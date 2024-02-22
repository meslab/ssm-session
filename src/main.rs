use clap::Parser;

use log::{debug, info};
use std::{collections::HashMap, str::FromStr};
use tokio;

use aws_sdk_ecs::model::{
    DescribeContainerInstancesRequest, DescribeTasksRequest, ListServicesRequest, ListTasksRequest,
};
use aws_sdk_ecs::{Client as EcsClient, Config as EcsConfig};
use aws_sdk_ssm::model::StartSessionRequest;
use aws_sdk_ssm::{Client as SsmClient, Config as SsmConfig};

#[derive(Parcer)]
#[command(author, version, about, long_about = None)]
#[clap(
    version = "1.0",
    author = "Your Name",
    about = "Starts a session on an ECS service instance"
)]
struct Args {
    #[clap(short, long, value_name = "SERVICE", about = "Sets the service name")]
    service: String,

    #[clap(short, long, value_name = "CLUSTER", about = "Sets the cluster name")]
    cluster: String,

    #[clap(short, long, value_name = "REGION", about = "Sets the region")]
    region: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args = Args::parse();

    let service = args.service;
    let cluster = args.cluster;
    let region = args.region;

    let ecs_config = EcsConfig::builder().region(region.to_string()).build();
    let ecs_client = EcsClient::from_conf(ecs_config);

    let ssm_config = SsmConfig::builder().region(region.to_string()).build();
    let ssm_client = SsmClient::from_conf(ssm_config);

    let mut list_services_request = ListServicesRequest::default();
    list_services_request.cluster = Some(cluster.clone());
    list_services_request.max_results = Some(100);

    let mut list_services_result = ecs_client
        .list_services(list_services_request.clone())
        .await?;
    let mut services = list_services_result.service_arns.unwrap_or_default();

    while let Some(next_token) = list_services_result.next_token {
        list_services_request.next_token = Some(next_token.clone());
        list_services_result = ecs_client
            .list_services(list_services_request.clone())
            .await?;
        services.extend(list_services_result.service_arns.unwrap_or_default());
    }

    debug!("List services result: {:?}", services);

    if let Some(service_arn) = services.into_iter().find(|arn| arn.contains(&service)) {
        info!("Service ARN: {:?}", service_arn);

        let list_tasks_request = ListTasksRequest {
            cluster: Some(cluster.clone()),
            service_name: Some(service_arn.clone()),
            ..Default::default()
        };

        if let Some(task_arn) = ecs_client
            .list_tasks(list_tasks_request)
            .await?
            .task_arns
            .and_then(|mut arns| arns.pop())
        {
            let describe_tasks_request = DescribeTasksRequest {
                cluster: Some(cluster.clone()),
                tasks: vec![task_arn.clone()],
                ..Default::default()
            };

            if let Some(container_instance_arn) = ecs_client
                .describe_tasks(describe_tasks_request)
                .await?
                .tasks
                .and_then(|mut tasks| tasks.pop())
                .map(|task| task.container_instance_arn)
            {
                let describe_container_instances_request = DescribeContainerInstancesRequest {
                    cluster: Some(cluster.clone()),
                    container_instances: vec![container_instance_arn.clone().expect("REASON")],
                    ..Default::default()
                };

                if let Some(instance_id) = ecs_client
                    .describe_container_instances(describe_container_instances_request)
                    .await?
                    .container_instances
                    .and_then(|mut instances| instances.pop())
                    .map(|instance| instance.ec_2_instance_id)
                {
                    let mut params = HashMap::new();
                    params.insert("command".to_string(), vec!["sudo su".to_string()]);

                    let start_session_request = StartSessionRequest {
                        target: instance_id.expect("REASON"),
                        document_name: Some("AWS-StartInteractiveCommand".to_string()),
                        parameters: Some(params),
                        ..Default::default()
                    };

                    info!("Start session request: {:?}", start_session_request);
                    info!("Starting session on {}", instance_id.expect("REASON"));
                    ssm_client.start_session(start_session_request);
                }
            }
        }
    }

    Ok(())
}
