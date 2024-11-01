use xcfg::XCfg;

use super::{
    group::TaskGroupConfig,
    user::{HostDeviceConfig, SSHDeviceConfig, SSHUserConfig},
    AppTaskConfig, AutoTaskConfig, Cite, CopyTaskConfig, ExecTaskConfig, FullConfig, Target,
    TaskAttr,
};

pub fn example(fmt: xcfg::Format) -> String {
    let id = "example_id".to_string();
    let host = Some(HostDeviceConfig::default().system(SSHUserConfig::new("system", "system")));
    let ssh = vec![SSHDeviceConfig::new("r0", [])
        .os("debian")
        .root(SSHUserConfig::new("r0_root", "r").passwd("passwd"))];
    let auto = vec![
        AutoTaskConfig::new(
            TaskAttr::new("service setup").with_next(["service reload"]),
            "a.service",
            "setup",
        ),
        AutoTaskConfig::new("service reload", "a.service", "reload"),
    ];
    let group = vec![
        TaskGroupConfig::new("this")
            .with_target(Target::default().with_src("host").with_dst("system"))
            .with_cites([
                Cite::new("service setup").with_target(Target::default().with_dst("system")),
                Cite::new("service reload"),
            ])
            .with_copy([
                CopyTaskConfig::new(
                    TaskAttr::new("service config").with_next(["service setup"]),
                    [("service/config", "/etc/service")],
                ),
                CopyTaskConfig::new("app config", [("app", "~/.config/app")]).with_dst("host"),
            ])
            .with_app([AppTaskConfig::new("install pkg", vec!["pkg"])]), // .with_shell([ShellTaskConfig::new(
        TaskGroupConfig::new("rsetuc a")
            .with_target(Target::default().with_dst("r0"))
            .with_copy([CopyTaskConfig::new("a config", [("a", "~/.config/a")]).with_src("host")]),
        TaskGroupConfig::new("rsetup b")
            .with_target(Target::default().with_dst("r0"))
            .with_cites([
                Cite::new("service setup"),
                Cite::new("service reload"),
                Cite::new("rsetup a").with_next(["service"]),
            ])
            .with_auto([AutoTaskConfig::new("service", "a.service", "reload").with_user("host")])
            .with_app([AppTaskConfig::new("service install", ["service"]).with_user("host")])
            .with_exec([ExecTaskConfig::new(
                TaskAttr::new("exec b").with_next(["rsetup a"]),
                "echo b && exit",
            )
            .shell("bash")
            .with_user("host")]),
    ];
    FullConfig {
        id,
        host,
        ssh,
        group,
        auto,
        ..Default::default()
    }
    .fmt_to_string(fmt)
    .expect("can't format config")
}
