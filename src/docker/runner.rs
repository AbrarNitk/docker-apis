use crate::running::remove;
use bollard::container::{Config, CreateContainerOptions, StartContainerOptions};
use bollard::image::CreateImageOptions;
use bollard::models::{
    ContainerState, ContainerStateStatusEnum, Health, HealthConfig, HealthStatusEnum, HostConfig,
    PortBinding,
};
use bollard::Docker;
use futures::StreamExt;
use std::collections::HashMap;

pub struct ContainerRunner {
    name: String,
    docker: Docker,
    image: String,
    port_bindings: Vec<(u16, u16)>,
    env_vars: Vec<(String, String)>,
    healthcheck: Option<HealthConfig>,
}

pub struct ContainerRunnerBuilder {
    name: String,
    image: Option<String>,
    port_bindings: Vec<(u16, u16)>,
    env_vars: Vec<(String, String)>,
    healthcheck: Option<HealthConfig>,
}

impl ContainerRunnerBuilder {
    pub fn new(name: String) -> Self {
        ContainerRunnerBuilder {
            name,
            image: None,
            port_bindings: Vec::new(),
            env_vars: Vec::new(),
            healthcheck: None,
        }
    }

    pub fn image(mut self, image: String) -> Self {
        self.image = Some(image);
        self
    }

    pub fn add_port_binding(mut self, host_port: u16, container_port: u16) -> Self {
        self.port_bindings.push((host_port, container_port));
        self
    }

    pub fn add_env_var(mut self, key: &str, value: &str) -> Self {
        self.env_vars.push((key.to_owned(), value.to_owned()));
        self
    }

    pub fn healthcheck(mut self, health_config: HealthConfig) -> Self {
        self.healthcheck = Some(health_config);
        self
    }

    pub fn build(self) -> anyhow::Result<ContainerRunner> {
        let image = self
            .image
            .ok_or_else(|| anyhow::anyhow!("Image must be set"))?;
        Ok(ContainerRunner {
            image,
            docker: Docker::connect_with_local_defaults()?,
            name: self.name,
            port_bindings: self.port_bindings,
            env_vars: self.env_vars,
            healthcheck: self.healthcheck,
        })
    }
}

impl ContainerRunner {
    pub async fn run(self) -> anyhow::Result<super::running::RunningContainer> {
        if self.is_running().await? {
            println!("removing already running container: {}", self.name);
            remove(&self.docker, self.name.as_str()).await?;
            println!("removed already running container: {}", self.name);
        }

        self.pull_image().await?;

        let host_config = Some(HostConfig {
            port_bindings: self.port_bindings(),
            ..Default::default()
        });

        let envs = self
            .env_vars
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>();

        let config = Config {
            image: Some(self.image),
            env: Some(envs),
            host_config,
            healthcheck: self.healthcheck,
            attach_stdout: Some(true),
            ..Default::default()
        };

        let options = CreateContainerOptions {
            name: self.name.clone(),
            platform: None,
        };

        let _ = self.docker.create_container(Some(options), config).await?;

        self.docker
            .start_container(self.name.as_str(), None::<StartContainerOptions<String>>)
            .await?;

        let start = std::time::Instant::now();
        loop {
            let inspect_container = self.docker.inspect_container(&self.name, None).await?;
            if let Some(ContainerState {
                status: Some(ContainerStateStatusEnum::RUNNING),
                health:
                    Some(Health {
                        status: Some(HealthStatusEnum::HEALTHY),
                        ..
                    }),
                ..
            }) = inspect_container.state
            {
                println!("Container is running and healthy");
                break;
            }

            if start.elapsed().as_secs() > 30 {
                return Err(anyhow::anyhow!("container failed to start"));
            }

            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        Ok(crate::running::RunningContainer {
            name: self.name,
            docker: self.docker,
        })
    }

    async fn pull_image(&self) -> anyhow::Result<()> {
        let available_images = self.docker.list_images::<String>(None).await?;
        for image in available_images {
            let exists = image.repo_tags.iter().any(|t| t.eq(&self.name));
            if exists {
                println!("Docker image is already pulled: {}", self.image);
                return Ok(());
            }
        }

        let create_options = CreateImageOptions::<&str> {
            from_image: &self.image.as_str(),
            ..Default::default()
        };

        let mut pull = self.docker.create_image(Some(create_options), None, None);

        while let Some(event) = pull.next().await {
            println!("Pulling Image: {:?}", event?);
        }

        Ok(())
    }

    async fn is_running(&self) -> anyhow::Result<bool> {
        let containers = self.docker.list_containers::<&str>(None).await?;
        for container in containers {
            let names = match container.names {
                Some(names) => names,
                None => continue,
            };

            let any_running = names
                .iter()
                .any(|name| name.eq(&self.name) || name.eq(&format!("/{}", self.name)));

            if any_running {
                println!("container is already running container: {}", self.name);
                return Ok(true);
            }
        }

        Ok(false)
    }
}

impl ContainerRunner {
    fn port_bindings(&self) -> Option<HashMap<String, Option<Vec<PortBinding>>>> {
        let port_bindings: HashMap<_, _> = self
            .port_bindings
            .iter()
            .map(|(hp, cp)| {
                (
                    format!("{cp}/tcp"),
                    Some(vec![PortBinding {
                        host_ip: Some("127.0.0.1".to_string()),
                        host_port: Some(format!("{hp}/tcp")),
                    }]),
                )
            })
            .collect();

        if port_bindings.is_empty() {
            None
        } else {
            Some(port_bindings)
        }
    }
}
