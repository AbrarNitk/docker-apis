use bollard::container::{RemoveContainerOptions, StartContainerOptions};
use bollard::Docker;

pub mod running;

pub struct RunningContainer {
    pub name: String,
    docker: Docker,
}

impl RunningContainer {
    pub async fn start(&self) -> anyhow::Result<()> {
        Ok(())
    }

    pub async fn remove(&self) -> anyhow::Result<()> {
        remove(&self.docker, self.name.as_str()).await?;
        Ok(())
    }

    pub async fn stop(&self) -> anyhow::Result<()> {
        Ok(())
    }

    pub async fn restart(&self) -> anyhow::Result<()> {
        self.stop().await?;
        self.start().await?;
        Ok(())
    }
}

pub async fn remove(docker: &Docker, name: &str) -> anyhow::Result<()> {
    docker
        .remove_container(
            name,
            Some(RemoveContainerOptions {
                force: true,
                ..Default::default()
            }),
        )
        .await?;
    Ok(())
}

pub async fn stop(docker: &Docker, name: &str) -> anyhow::Result<()> {
    docker.stop_container(name, None).await?;
    Ok(())
}

pub async fn start(docker: &Docker, name: &str) -> anyhow::Result<()> {
    docker
        .start_container(name, None::<StartContainerOptions<String>>)
        .await?;
    Ok(())
}
