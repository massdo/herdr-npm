use cucumber::World;
use herdr_npm::domain::error::AppError;
use herdr_npm::domain::ids::PaneId;
use herdr_npm::domain::pane::OriginContext;

use super::fake_herdr::FakeHerdr;
use super::fake_project::FakeProject;

#[derive(Debug, Default, World)]
pub struct BddWorld {
    pub herdr: FakeHerdr,
    #[allow(dead_code)]
    pub project: FakeProject,
    pub origin: Option<OriginContext>,
    pub last_error: Option<AppError>,
    pub opened_pane: Option<PaneId>,
    pub plugin_id: Option<String>,
}
