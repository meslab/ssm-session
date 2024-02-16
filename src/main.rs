use log::{debug, info};
use rusoto_core::Region;
use rusoto_ecs::{
    DescribeContainerInstancesRequest, DescribeTasksRequest, Ecs, EcsClient, ListServicesRequest,
    ListTasksRequest,
};
use rusoto_ssm::{Ssm, SsmClient, StartSessionRequest};
use std::{collections::HashMap, str::FromStr};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let service = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "auth".to_string());
    let cluster = std::env::args().nth(2).unwrap_or_else(|| "app".to_string());
    let region = std::env::args()
        .nth(3)
        .unwrap_or_else(|| "EuCentral1".to_string());

    let ecs_client = EcsClient::new(Region::from_str(&region)?);
    let ssm_client = SsmClient::new(Region::from_str(&region)?);

    let mut list_services_request = ListServicesRequest {
        cluster: Some(cluster.clone()),
        max_results: Some(100),
        ..Default::default()
    };

    let mut list_services_result = ecs_client
        .list_services(list_services_request.clone())
        .await?;
    let mut services = list_services_result
        .service_arns
        .clone()
        .unwrap_or_else(Vec::new);

    while let Some(next_token) = list_services_result.next_token {
        list_services_request.next_token = Some(next_token.clone());
        list_services_result = ecs_client
            .list_services(list_services_request.clone())
            .await?;
        services.extend(
            list_services_result
                .service_arns
                .clone()
                .unwrap_or_else(Vec::new),
        );
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
            .and_then(|arns| arns.into_iter().next())
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
                .and_then(|tasks| {
                    tasks
                        .into_iter()
                        .next()
                        .map(|task| task.container_instance_arn)
                })
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
                    .and_then(|instances| {
                        instances
                            .into_iter()
                            .next()
                            .map(|instance| instance.ec_2_instance_id)
                    })
                {
                    let start_session_request = StartSessionRequest {
                        target: instance_id.clone().expect("REASON"),
                        document_name: Some("AWS-StartInteractiveCommand".to_string()),
                        parameters: Some(HashMap::from([(
                            "command".to_string(),
                            vec!["sudo su".to_string()],
                        )])),
                        ..Default::default()
                    };
                    info!("Start session request: {:?}", start_session_request);
                    info!(
                        "Starting session on {}",
                        instance_id.clone().expect("REASON")
                    );
                    ssm_client.start_session(start_session_request).await?;
                }
            }
        }
    }

    Ok(())
}
