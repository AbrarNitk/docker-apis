use bollard::image::CreateImageOptions;
use bollard::models::HealthConfig;
use bollard::Docker;
use futures::StreamExt;

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
    pub fn new(name: &str) -> Self {
        ContainerRunnerBuilder {
            name: name.to_string(),
            image: None,
            port_bindings: Vec::new(),
            env_vars: Vec::new(),
            healthcheck: None,
        }
    }

    pub fn image(mut self, image: &str) -> Self {
        self.image = Some(image.to_owned());
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
}
