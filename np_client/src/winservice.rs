use anyhow::Context;
use clap::Parser;
use log::{debug, error, info};
use std::env;
use std::ffi::OsString;
use std::thread;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::select;
use tokio::sync::oneshot;

use crate::{init_logger, run_with_args, Commands, CommonArgs, Opts};
use windows_service::{
    define_windows_service,
    service::{
        ServiceAccess, ServiceControl, ServiceControlAccept, ServiceErrorControl, ServiceExitCode,
        ServiceInfo, ServiceStartType, ServiceState, ServiceStatus, ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult, ServiceStatusHandle},
    service_dispatcher,
    service_manager::{ServiceManager, ServiceManagerAccess},
};

const SERVICE_NAME: &str = "np_client";
const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;
const SERVICE_EXE: &str = "np_client.exe";
const SERVICE_DESC: &str = "net pipe client";
const SERVICE_DISPLAY_NAME: &str = "npipe client";

// Generate the Windows Service boilerplate.
// The boilerplate contains the low-level service entry function (ffi_service_main)
// that parses incoming service arguments into Vec<OsString> and passes them to
// user defined service entry (custom_service_main).
define_windows_service!(ffi_service_main, custom_service_main);

fn custom_service_main(_args: Vec<OsString>) {
    let runtime = Runtime::new().expect("Failed to create Tokio runtime");
    runtime.block_on(async {
        if let Err(err) = run_service().await {
            error!("error starting the service: {:?}", err);
        }
    });
}

/// Assigns a particular server state with its properties.
fn set_service_state(
    status_handle: &ServiceStatusHandle,
    current_state: ServiceState,
    checkpoint: u32,
    wait_hint: Duration,
) -> anyhow::Result<()> {
    let next_status = ServiceStatus {
        // Should match the one from system service registry
        service_type: SERVICE_TYPE,
        // The new state
        current_state,
        // Accept stop events when running
        controls_accepted: ServiceControlAccept::STOP,
        // Used to report an error when starting or stopping only, otherwise must be zero
        exit_code: ServiceExitCode::Win32(0),
        // Only used for pending states, otherwise must be zero
        checkpoint,
        // Only used for pending states, otherwise must be zero
        wait_hint,
        // Unused for setting status
        process_id: None,
    };

    // Inform the system about the service status
    Ok(status_handle.set_service_status(next_status)?)
}

async fn run_service() -> anyhow::Result<()> {
    // Log is already initialized so there is no need to do it again.
    let ops = Opts::parse();
    info!("windows service: starting service setup");

    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    let mut shutdown_tx = Some(shutdown_tx);

    // Define system service event handler that will be receiving service events.
    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            // Notifies a service to report its current status information to the service
            // control manager. Always return NoError even if not implemented.
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,

            // Handle stop
            ServiceControl::Stop => {
                debug!("windows service: handled 'ServiceControl::Stop' event");
                if let Some(sender) = shutdown_tx.take() {
                    debug!("windows service: delegated 'ServiceControl::Stop' event");
                    tokio::spawn(async move {
                        let _ = sender.send(());
                    });
                }
                ServiceControlHandlerResult::NoError
            }

            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    // Register system service event handler.
    // The returned status handle should be used to report service status changes to the system.
    let status_handle = service_control_handler::register(SERVICE_NAME, event_handler)?;
    info!("windows service: registering service");

    // Service is running
    set_service_state(
        &status_handle,
        ServiceState::Running,
        1,
        Duration::default(),
    )?;
    info!("windows service: set service 'Running' state");

    match ops.command {
        Some(Commands::RunService { common_args }) => {
            let run_task = async {
                if let Err(err) = run_with_args(common_args).await {
                    error!(
                        "windows service: error after starting the server: {:?}",
                        err
                    );
                }
            };

            select! {
                _= run_task =>{},
                _= shutdown_rx =>{},
            };

            match set_service_state(
                &status_handle,
                ServiceState::StopPending,
                2,
                Duration::from_secs(3),
            ) {
                Ok(()) => info!("windows service: set service 'StopPending' state"),
                Err(err) => error!(
                    "windows service: error when setting 'StopPending' state: {:?}",
                    err
                ),
            }
        }
        _ => {
            println!("Wrong run command")
        }
    }

    // Service is stopped
    set_service_state(
        &status_handle,
        ServiceState::Stopped,
        3,
        Duration::from_secs(3),
    )?;
    info!("windows service: set service 'Stopped' state");

    Ok(())
}

/// Run web server as Windows Server
pub fn run_server_as_service(common_args: CommonArgs) -> anyhow::Result<()> {
    // Set current directory to the same as the executable
    let mut path = env::current_exe().unwrap();
    path.pop();
    env::set_current_dir(&path).unwrap();

    init_logger(&common_args)?;

    // Register generated `ffi_service_main` with the system and start the
    // service, blocking this thread until the service is stopped
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)
        .with_context(|| "error registering generated `ffi_service_main` with the system")?;
    Ok(())
}

pub fn install_service(common_args: CommonArgs) -> anyhow::Result<()> {
    let manager_access = ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    // Set the executable path to point the current binary
    let service_binary_path = std::env::current_exe().unwrap().with_file_name(SERVICE_EXE);

    // Set service binary default arguments
    let mut service_binary_arguments = vec![
        OsString::from("run-service"),
        OsString::from(if common_args.backtrace {
            "--backtrace=true"
        } else {
            "--backtrace=false"
        }),
        OsString::from(format!("--server={}", common_args.server)),
        OsString::from(format!("--username={}", common_args.username)),
        OsString::from(format!("--password={}", common_args.password)),
        OsString::from(format!("--log-level={}", common_args.log_level)),
        OsString::from(format!("--base-log-level={}", common_args.base_log_level)),
        OsString::from(format!("--log-dir={}", common_args.log_dir)),
        OsString::from(format!("--ca-cert={}", common_args.ca_cert)),
        OsString::from(format!("--tls-server-name={}", common_args.tls_server_name)),
    ];

    if common_args.enable_tls {
        service_binary_arguments.push(OsString::from("--enable-tls"));
    }
    if common_args.insecure {
        service_binary_arguments.push(OsString::from("--insecure"));
    }
    if common_args.quiet {
        service_binary_arguments.push(OsString::from("--quiet"));
    }

    // Run the current service as `System` type
    let service_info = ServiceInfo {
        name: OsString::from(SERVICE_NAME),
        display_name: OsString::from(SERVICE_DISPLAY_NAME),
        service_type: SERVICE_TYPE,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: service_binary_path,
        launch_arguments: service_binary_arguments,
        dependencies: vec![],
        account_name: None, // run as System
        account_password: None,
    };

    let service = service_manager.create_service(&service_info, ServiceAccess::CHANGE_CONFIG)?;
    service.set_description(SERVICE_DESC)?;

    println!(
        "Windows Service ({}) is installed successfully!",
        SERVICE_NAME
    );
    println!(
        "Start the service typing: sc.exe start \"{}\" (it requires administrator privileges) or using the 'services.msc' application.",
        SERVICE_NAME
    );

    Ok(())
}

/// Uninstall the current Windows Service for SWS.
pub fn uninstall_service() -> anyhow::Result<()> {
    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_access = ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE;
    let service = service_manager.open_service(SERVICE_NAME, service_access)?;

    let service_status = service.query_status()?;
    if service_status.current_state != ServiceState::Stopped {
        service.stop()?;
        // Wait for service to stop
        thread::sleep(Duration::from_secs(1));
    }

    service.delete()?;

    println!("Windows Service ({}) is uninstalled!", SERVICE_NAME);

    Ok(())
}
