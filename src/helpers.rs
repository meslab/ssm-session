use aws_config::default_provider::credentials::DefaultCredentialsChain;
use aws_sdk_ecs::config::Region;
use aws_sdk_ecs::{Client, Config};
use log::debug;

pub async fn initialize_client(region: &str, profile: &str) -> Client {
    let region = Region::new(region.to_owned());

    let credentials_provider = DefaultCredentialsChain::builder()
        .region(region.clone())
        .profile_name(profile)
        .build()
        .await;
    let ecs_config = Config::builder()
        .credentials_provider(credentials_provider)
        .region(region.clone())
        .build();

    Client::from_conf(ecs_config)
}

pub async fn get_service_arn(
    ecs_client: &Client,
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

pub async fn get_task_arn(
    ecs_client: &Client,
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

pub async fn get_task_container_arn(
    ecs_client: &Client,
    cluster: &String,
    task_arn: &String,
) -> Result<String, Box<dyn std::error::Error>> {
    let describe_tasks_result = ecs_client
        .describe_tasks()
        .cluster(cluster)
        .tasks(task_arn)
        .send()
        .await?;
    Ok(describe_tasks_result
        .tasks
        .expect("No task found!")
        .pop()
        .unwrap()
        .container_instance_arn
        .expect("No container instance found!"))
}

pub async fn get_container_arn(
    ecs_client: &Client,
    cluster: &String,
    container_instance_arn: &String,
) -> Result<String, Box<dyn std::error::Error>> {
    let describe_container_instances_result = ecs_client
        .describe_container_instances()
        .cluster(cluster)
        .container_instances(container_instance_arn)
        .send()
        .await?;
    Ok(describe_container_instances_result
        .container_instances
        .expect("No container instance found!")
        .pop()
        .unwrap()
        .ec2_instance_id
        .expect("No EC2 instance found!"))
}
