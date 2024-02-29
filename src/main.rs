use clap::Parser;
use log::info;
use std::process::Command;
mod helpers;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[clap(
    version = "v0.1.2",
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

    #[clap(short, long, default_value = "default")]
    profile: String,

    #[clap(short, long)]
    exec: Option<String>,

    #[clap(short, long)]
    instance: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let args = Args::parse();

    let mut command = format!(
        "command=sudo docker exec -ti $(sudo docker ps -qf name={} | head -n1) /bin/bash",
        &args.service
    );

    if let Some(exec) = args.exec {
        command = format!(
            "command=sudo docker exec -ti $(sudo docker ps -qf name={} | head -n1) /bin/bash -lc {}",
            &args.service, exec
        );
    }

    let ecs_client = helpers::initialize_client(&args.region, &args.profile).await;
    let instance_id;

    match args.instance {
        Some(instance) => {
            command = "command=sudo su -".to_string();
            instance_id = instance;
        }
        None => {
            let service_arn =
                helpers::get_service_arn(&ecs_client, &args.cluster, &args.service).await?;

            info!("Service ARN: {}", service_arn);

            let task_arn = helpers::get_task_arn(&ecs_client, &args.cluster, &service_arn).await?;

            info!("Task ARN: {}", task_arn);

            let task_instance_arn =
                helpers::get_task_container_arn(&ecs_client, &args.cluster, &task_arn).await?;
            info!("Task Instance ARN: {:?}", task_instance_arn);

            instance_id =
                helpers::get_container_arn(&ecs_client, &args.cluster, &task_instance_arn).await?;
            info!("Instance ID: {:?}", instance_id);
        }
    }

    println!(
        "Service {} is running on instance {}",
        &args.service, instance_id
    );

    let mut ssm_session = Command::new("aws")
        .arg("ssm")
        .arg("start-session")
        .arg("--region")
        .arg(&args.region)
        .arg("--target")
        .arg(instance_id)
        .arg("--document-name")
        .arg("AWS-StartInteractiveCommand")
        .arg("--parameters")
        .arg("--profile")
        .arg(&args.profile)
        .arg(command)
        .spawn()
        .expect("Failed to start ssm session");

    let _ = ssm_session.wait().expect("Failed to wait for ssm session");

    Ok(())
}
