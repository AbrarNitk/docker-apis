use crate::runner::ContainerRunnerBuilder;
use crate::running::RunningContainer;
use bollard::models::HealthConfig;

pub async fn running_container(
    container_name: &str,
    port: u16,
    container_reg: Option<String>,
) -> anyhow::Result<RunningContainer> {
    let container_name = format!("pg_{}-{}", container_name, port);
    let docker_image = docker_image(container_reg);

    let running_container = ContainerRunnerBuilder::new(container_name)
        .image(docker_image)
        .add_port_binding(5432, port)
        .add_env_var("POSTGRES_PASSWORD", "postgres-password")
        .healthcheck(HealthConfig {
            test: Some(vec![
                "CMD-SHELL".to_string(),
                "pg_isready -U postgres".to_string(),
            ]),
            interval: Some(250_000_000), // 250ms
            timeout: Some(100_000_000),  // 100ms
            retries: Some(5),
            start_period: Some(500_000_000),
            start_interval: None,
        })
        .build()?
        .run()
        .await?;

    Ok(running_container)
}

fn docker_image(container_reg: Option<String>) -> String {
    let cg = container_reg.unwrap_or_else(|| container_registry());
    std::env::var("PG_DOCKER_IMAGE").unwrap_or_else(|_| format!("{}postgres:latest", cg))
}

fn container_registry() -> String {
    std::env::var("PG_CONTAINER_REGISTRY")
        .unwrap_or_else(|_| "public.ecr.aws/docker/library/".to_string())
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn test_pg() {
        let rc = super::running_container("demo-container", 5432, None)
            .await
            .expect("cannot create the container");
        println!("container is running");
        rc.stop().await.unwrap();
        rc.remove().await.unwrap();
    }
}
