use aws_config::default_provider::credentials::DefaultCredentialsChain;
use aws_sdk_ecs::config::Region as EcsRegion;
use aws_sdk_ecs::{Client as EcsClient, Config as EcsConfig};
use clap::Parser;
use log::{debug, info};
use tokio;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[clap(
    version = "v0.0.1",
    author = "Anton Sidorov tonysidrock@gmail.com",
    about = "Counts wwords frequency in a text file"
)]
struct Args {
    #[clap(short, long, default_value = "auth")]
    service: String,

    #[clap(short, long, default_value = "app")]
    cluster: String,

    #[clap(short, long, default_value = "eu-central-1")]
    region: String,
}

async fn get_service_arn(
    ecs_client: &EcsClient,
    cluster: &String,
    service: &String,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut ecs_services_stream = ecs_client
        .list_services()
        .cluster(cluster)
        .max_results(100)
        .into_paginator()
        .send();

    while let Some(services) = ecs_services_stream.next().await {
        debug!("Services: {:?}", services);
        let service_arn = services
            .unwrap()
            .service_arns
            .unwrap()
            .into_iter()
            .find(|arn| arn.contains(service));
        if let Some(service_arn) = service_arn {
            debug!("Inside get_service_arn Service ARN: {:?}", service_arn);
            return Ok(service_arn);
        }
    }
    Err("Service not found".into())
}

async fn get_task_arn(
    ecs_client: &EcsClient,
    cluster: &String,
    service: &String,
) -> Result<String, Box<dyn std::error::Error>> {
    let list_tasks_result = ecs_client
        .list_tasks()
        .cluster(cluster)
        .service_name(service)
        .send()
        .await?;
    Ok(list_tasks_result.task_arns.unwrap().pop().unwrap())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args = Args::parse();

    let region = EcsRegion::new(args.region.clone());

    let credentials_provider = DefaultCredentialsChain::builder()
        .region(region.clone())
        .build()
        .await;
    let ecs_config = EcsConfig::builder()
        .credentials_provider(credentials_provider)
        .region(region.clone())
        .build();

    debug!(
        "Cluster: {}, Service: {}, Region: {}.",
        &args.cluster, &args.service, &args.region
    );

    let ecs_client = EcsClient::from_conf(ecs_config);
    let service_arn = get_service_arn(&ecs_client, &args.cluster, &args.service).await?;

    info!("Service ARN: {}", service_arn);

    let task_arn = get_task_arn(&ecs_client, &args.cluster, &service_arn).await?;

    info!("Task ARN: {}", task_arn);

    //    if let Some(service_arn) = services.into_iter().find(|arn| arn.contains(&service)) {
    //        info!("Service ARN: {:?}", service_arn);
    //
    //        let list_tasks_request = ListTasksRequest {
    //            cluster: Some(cluster.clone()),
    //            service_name: Some(service_arn.clone()),
    //            ..Default::default()
    //        };
    //
    //        let list_tasks_result = ecs_client.list_tasks(list_tasks_request).await?;
    //
    //        if let Some(task_arn) = list_tasks_result.task_arns.and_then(|mut arns| arns.pop()) {
    //            let describe_tasks_request = DescribeTasksRequest {
    //                cluster: Some(cluster.clone()),
    //                tasks: vec![task_arn.clone()],
    //                ..Default::default()
    //            };
    //
    //            let describe_tasks_result = ecs_client.describe_tasks(describe_tasks_request).await?;
    //
    //            if let Some(container_instance_arn) = describe_tasks_result.tasks.and_then(|mut tasks| tasks.pop()).map(|task| task.container_instance_arn) {
    //                let describe_container_instances_request = DescribeContainerInstancesRequest {
    //                    cluster: Some(cluster.clone()),
    //                    container_instances: vec![container_instance_arn.clone().expect("REASON")],
    //                    ..Default::default()
    //                };
    //
    //                let describe_container_instances_result = ecs_client.describe_container_instances(describe_container_instances_request).await?;
    //
    //                if let Some(instance_id) = describe_container_instances_result.container_instances.and_then(|mut instances| instances.pop()).map(|instance| instance.ec_2_instance_id) {
    //                    let mut params = HashMap::new();
    //                    params.insert("command".to_string(), vec!["sudo su".to_string()]);
    //
    //                    let start_session_request = StartSessionRequest {
    //                        target: instance_id.expect("REASON"),
    //                        document_name: Some("AWS-StartInteractiveCommand".to_string()),
    //                        parameters: Some(params),
    //                        ..Default::default()
    //                    };
    //
    //                    info!("Start session request: {:?}", start_session_request);
    //                    info!("Starting session on {}", instance_id.expect("REASON"));
    //                    ssm_client.start_session();
    //                }
    //            }
    //        }
    //    }
    //
    Ok(())
}
