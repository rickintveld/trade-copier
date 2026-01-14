use anyhow::Result;
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

use crate::database::{Database, DependencyStatus};

#[derive(Debug, Clone, Serialize)]
pub struct DependencyStatusResponse {
    pub os_name: String,
    pub os_version: Option<String>,
    pub package_manager_name: String,
    pub package_manager_status: String,
    pub wine_status: String,
    pub error_message: Option<String>,
    pub all_installed: bool,
}

/// Get OS information
fn get_os_info() -> (String, Option<String>) {
    #[cfg(target_os = "macos")]
    {
        ("macOS".to_string(), sys_info::os_release().ok())
    }
    
    #[cfg(target_os = "windows")]
    {
        ("Windows".to_string(), sys_info::os_release().ok())
    }
    
    #[cfg(target_os = "linux")]
    {
        ("Linux".to_string(), sys_info::os_release().ok())
    }
}

/// Get package manager name for the current OS
fn get_package_manager_name() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "Homebrew"
    }
    
    #[cfg(target_os = "windows")]
    {
        "Chocolatey"
    }
    
    #[cfg(target_os = "linux")]
    {
        "apt/yum"
    }
}

/// Check if package manager is installed
fn is_package_manager_installed() -> bool {
    #[cfg(target_os = "macos")]
    {
        crate::installer::package_manager::is_homebrew_installed()
    }
    
    #[cfg(target_os = "windows")]
    {
        crate::installer::package_manager::is_chocolatey_installed()
    }
    
    #[cfg(target_os = "linux")]
    {
        true // Most Linux systems have apt or yum pre-installed
    }
}

/// Check if Wine is installed
fn is_wine_installed() -> bool {
    #[cfg(target_os = "macos")]
    {
        crate::installer::package_manager::is_wine_installed()
    }
    
    #[cfg(target_os = "windows")]
    {
        true // Wine not needed on Windows
    }
    
    #[cfg(target_os = "linux")]
    {
        // For Linux, would need similar check as macOS
        true
    }
}

/// Check all dependencies and update database
pub async fn check_dependencies(db: Arc<Database>, app_handle: Option<&AppHandle>) -> Result<DependencyStatusResponse> {
    let (os_name, os_version) = get_os_info();
    let package_manager_name = get_package_manager_name();
    
    let pm_installed = is_package_manager_installed();
    let wine_installed = is_wine_installed();
    
    let pm_status = if pm_installed {
        DependencyStatus::Installed
    } else {
        DependencyStatus::Pending
    };
    
    let wine_status = if wine_installed {
        DependencyStatus::Installed
    } else if !pm_installed {
        // Can't install Wine without package manager
        DependencyStatus::Pending
    } else {
        DependencyStatus::Pending
    };
    
    // Update database
    db.upsert_system_dependencies(
        &os_name,
        os_version.as_deref(),
        package_manager_name,
        pm_status,
        wine_status,
        None,
    ).await?;
    
    let all_installed = pm_installed && wine_installed;
    
    let response = DependencyStatusResponse {
        os_name: os_name.clone(),
        os_version: os_version.clone(),
        package_manager_name: package_manager_name.to_string(),
        package_manager_status: pm_status.as_str().to_string(),
        wine_status: wine_status.as_str().to_string(),
        error_message: None,
        all_installed,
    };
    
    // Emit event if app_handle is provided
    if let Some(handle) = app_handle {
        let _ = handle.emit_all("dependency-status-changed", &response);
    }
    
    Ok(response)
}

/// Get current dependency status from database
pub async fn get_dependency_status(db: Arc<Database>) -> Result<Option<DependencyStatusResponse>> {
    match db.get_system_dependencies().await? {
        Some(record) => {
            let pm_status = DependencyStatus::from_str(&record.package_manager_status);
            let wine_status = DependencyStatus::from_str(&record.wine_status);
            
            let all_installed = pm_status == DependencyStatus::Installed 
                && wine_status == DependencyStatus::Installed;
            
            Ok(Some(DependencyStatusResponse {
                os_name: record.os_name,
                os_version: record.os_version,
                package_manager_name: record.package_manager_name,
                package_manager_status: record.package_manager_status,
                wine_status: record.wine_status,
                error_message: record.error_message,
                all_installed,
            }))
        }
        None => Ok(None),
    }
}

/// Install missing dependencies
pub async fn install_dependencies(db: Arc<Database>, app_handle: &AppHandle) -> Result<DependencyStatusResponse> {
    let (os_name, os_version) = get_os_info();
    let package_manager_name = get_package_manager_name();
    
    // Check current status
    let pm_installed = is_package_manager_installed();
    let wine_installed = is_wine_installed();
    
    // Install package manager if needed
    if !pm_installed {
        // Update status to installing
        db.upsert_system_dependencies(
            &os_name,
            os_version.as_deref(),
            package_manager_name,
            DependencyStatus::Installing,
            DependencyStatus::Pending,
            None,
        ).await?;
        
        let status = DependencyStatusResponse {
            os_name: os_name.clone(),
            os_version: os_version.clone(),
            package_manager_name: package_manager_name.to_string(),
            package_manager_status: "installing".to_string(),
            wine_status: "pending".to_string(),
            error_message: None,
            all_installed: false,
        };
        let _ = app_handle.emit_all("dependency-status-changed", &status);
        
        #[cfg(target_os = "macos")]
        {
            if let Err(e) = crate::installer::package_manager::install_homebrew() {
                let error_msg = format!("Failed to install Homebrew: {}", e);
                db.upsert_system_dependencies(
                    &os_name,
                    os_version.as_deref(),
                    package_manager_name,
                    DependencyStatus::Error,
                    DependencyStatus::Pending,
                    Some(&error_msg),
                ).await?;
                
                return Ok(DependencyStatusResponse {
                    os_name,
                    os_version,
                    package_manager_name: package_manager_name.to_string(),
                    package_manager_status: "error".to_string(),
                    wine_status: "pending".to_string(),
                    error_message: Some(error_msg),
                    all_installed: false,
                });
            }
        }
        
        #[cfg(target_os = "windows")]
        {
            if let Err(e) = crate::installer::package_manager::install_chocolatey() {
                let error_msg = format!("Failed to install Chocolatey: {}", e);
                db.upsert_system_dependencies(
                    &os_name,
                    os_version.as_deref(),
                    package_manager_name,
                    DependencyStatus::Error,
                    DependencyStatus::Pending,
                    Some(&error_msg),
                ).await?;
                
                return Ok(DependencyStatusResponse {
                    os_name,
                    os_version,
                    package_manager_name: package_manager_name.to_string(),
                    package_manager_status: "error".to_string(),
                    wine_status: "pending".to_string(),
                    error_message: Some(error_msg),
                    all_installed: false,
                });
            }
        }
        
        // Update to installed
        db.upsert_system_dependencies(
            &os_name,
            os_version.as_deref(),
            package_manager_name,
            DependencyStatus::Installed,
            DependencyStatus::Pending,
            None,
        ).await?;
        
        let status = DependencyStatusResponse {
            os_name: os_name.clone(),
            os_version: os_version.clone(),
            package_manager_name: package_manager_name.to_string(),
            package_manager_status: "installed".to_string(),
            wine_status: "pending".to_string(),
            error_message: None,
            all_installed: false,
        };
        let _ = app_handle.emit_all("dependency-status-changed", &status);
    }
    
    // Install Wine if needed (only on macOS/Linux)
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    if !wine_installed {
        // Update status to installing
        db.upsert_system_dependencies(
            &os_name,
            os_version.as_deref(),
            package_manager_name,
            DependencyStatus::Installed,
            DependencyStatus::Installing,
            None,
        ).await?;
        
        let status = DependencyStatusResponse {
            os_name: os_name.clone(),
            os_version: os_version.clone(),
            package_manager_name: package_manager_name.to_string(),
            package_manager_status: "installed".to_string(),
            wine_status: "installing".to_string(),
            error_message: None,
            all_installed: false,
        };
        let _ = app_handle.emit_all("dependency-status-changed", &status);
        
        #[cfg(target_os = "macos")]
        {
            if let Err(e) = crate::installer::package_manager::install_wine() {
                let error_msg = format!("Failed to install Wine: {}", e);
                db.upsert_system_dependencies(
                    &os_name,
                    os_version.as_deref(),
                    package_manager_name,
                    DependencyStatus::Installed,
                    DependencyStatus::Error,
                    Some(&error_msg),
                ).await?;
                
                return Ok(DependencyStatusResponse {
                    os_name,
                    os_version,
                    package_manager_name: package_manager_name.to_string(),
                    package_manager_status: "installed".to_string(),
                    wine_status: "error".to_string(),
                    error_message: Some(error_msg),
                    all_installed: false,
                });
            }
        }
        
        // Update to installed
        db.upsert_system_dependencies(
            &os_name,
            os_version.as_deref(),
            package_manager_name,
            DependencyStatus::Installed,
            DependencyStatus::Installed,
            None,
        ).await?;
    }
    
    // Final check
    check_dependencies(db, Some(app_handle)).await
}
