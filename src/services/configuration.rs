use crate::infrastructure::configuration::ApplicationConfiguration;

pub fn load_configuration(path: &str) -> anyhow::Result<ApplicationConfiguration> {
    ApplicationConfiguration::load_from_path(path)
}
