use tracing::{info, trace};
use zbus::Result as ZResult;
use zbus::{proxy, zvariant::OwnedObjectPath};

type EnableUnitFilesReply = (bool, Vec<(String, String, String)>);
#[proxy(
    interface = "org.freedesktop.systemd1.Manager",
    default_service = "org.freedesktop.systemd1",
    default_path = "/org/freedesktop/systemd1"
)]
trait Manager {
    fn get_unit(&self, name: &str) -> ZResult<OwnedObjectPath>;
    fn enable_unit_files(
        &self,
        files: Vec<&str>,
        runtime: bool,
        force: bool,
    ) -> ZResult<EnableUnitFilesReply>;
    fn start_unit(&self, name: &str, mode: &str) -> ZResult<OwnedObjectPath>;
    fn reload(&self) -> ZResult<()>;
    fn reload_unit(&self, name: &str, mode: &str) -> ZResult<OwnedObjectPath>;
    fn reload_or_restart_unit(&self, name: &str, mode: &str) -> ZResult<OwnedObjectPath>;
    fn get_unit_file_state(&self, name: &str) -> ZResult<String>;
}

#[proxy(
    interface = "org.freedesktop.systemd1.Unit",
    default_service = "org.freedesktop.systemd1",
    default_path = "/org/freedesktop/systemd1/unit"
)]
trait Unit {
    #[zbus(property)]
    fn active_state(&self) -> ZResult<String>;
    #[zbus(property)]
    fn unit_file_state(&self) -> ZResult<String>;
    #[zbus(property)]
    fn need_daemon_reload(&self) -> ZResult<bool>;
    #[zbus(property)]
    fn can_reload(&self) -> ZResult<bool>;
    fn reload_or_restart(&self, mode: &str) -> ZResult<OwnedObjectPath>;
    fn stop(&self, mode: &str) -> ZResult<OwnedObjectPath>;
    fn start(&self, mode: &str) -> ZResult<OwnedObjectPath>;
    fn restart(&self, mode: &str) -> ZResult<OwnedObjectPath>;
}
pub struct AutoX {
    is_system: bool,
    conn: zbus::Connection,
    manager: ManagerProxy<'static>,
}

impl AutoX {
    pub async fn new(is_system: bool) -> Result<Self, zbus::Error> {
        let conn = if is_system {
            info!("start system connection");
            zbus::Connection::system().await
        } else {
            info!("start user connection");
            zbus::Connection::session().await
        }?;
        let manager = ManagerProxy::new(&conn).await?;
        Ok(Self {
            is_system,
            conn,
            manager,
        })
    }
    pub async fn setup(&self, _name: &str, _args: &str) -> Result<(), zbus::Error> {
        unimplemented!()
    }
    pub async fn enable(&self, name: &str) -> Result<(), zbus::Error> {
        trace!(
            "[{}] setup {}",
            if self.is_system { "system" } else { "user" },
            name
        );
        let unit_path = self.manager.get_unit(name).await?;
        let unit = UnitProxy::builder(&self.conn)
            .path(unit_path)?
            .build()
            .await?;
        if unit.need_daemon_reload().await? {
            info!(
                "[{}] daemon reload",
                if self.is_system { "system" } else { "user" }
            );
            self.manager.reload().await?;
        }
        if unit.unit_file_state().await? != "enabled" {
            info!(
                "[{}] {} enabled",
                if self.is_system { "system" } else { "user" },
                name,
            );
            self.manager
                .enable_unit_files(vec![name], false, false)
                .await?;
        }
        Ok(())
    }
    pub async fn reload(&self, name: &str) -> Result<(), zbus::Error> {
        let unit_path = self.manager.get_unit(name).await?;
        let unit = UnitProxy::builder(&self.conn)
            .path(unit_path)?
            .build()
            .await?;
        if unit.can_reload().await? {
            unit.reload_or_restart("replace").await?;
        } else if unit.active_state().await? == "active" {
            unit.restart("replace").await?;
        } else {
            unit.start("replace").await?;
        }
        Ok(())
    }
}
