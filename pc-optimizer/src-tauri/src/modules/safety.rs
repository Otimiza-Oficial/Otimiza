// Safety Validator and Backup Manager
// Ensures all operations are safe and reversible

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub code: String,
    pub message: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    pub code: String,
    pub message: String,
    pub recommendation: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
    pub safety_score: u8, // 0-100
}

impl ValidationResult {
    pub fn failure(errors: Vec<ValidationError>) -> Self {
        ValidationResult {
            valid: false,
            errors,
            warnings: Vec::new(),
            safety_score: 0,
        }
    }
}

pub struct SafetyValidator {
    critical_services: Vec<String>,
    critical_registry_keys: Vec<String>,
}

impl SafetyValidator {
    pub fn new() -> Self {
        SafetyValidator {
            critical_services: Self::load_critical_services(),
            critical_registry_keys: Self::load_critical_registry_keys(),
        }
    }

    /// Carrega lista de serviços críticos que não devem ser modificados
    fn load_critical_services() -> Vec<String> {
        vec![
            "wuauserv".to_string(),    // Windows Update
            "BITS".to_string(),         // Background Intelligent Transfer Service
            "EventLog".to_string(),     // Windows Event Log
            "RpcSs".to_string(),        // Remote Procedure Call
            "DcomLaunch".to_string(),   // DCOM Server Process Launcher
            "PlugPlay".to_string(),     // Plug and Play
            "BFE".to_string(),          // Base Filtering Engine (Firewall)
            "mpssvc".to_string(),       // Windows Defender Firewall
        ]
    }

    /// Carrega lista de chaves de registro críticas
    fn load_critical_registry_keys() -> Vec<String> {
        vec![
            "HKLM\\SYSTEM\\CurrentControlSet\\Control".to_string(),
            "HKLM\\SYSTEM\\CurrentControlSet\\Services".to_string(),
            "HKLM\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Winlogon".to_string(),
        ]
    }

    /// Verifica se um serviço é crítico
    pub fn is_critical_service(&self, service_name: &str) -> bool {
        self.critical_services
            .iter()
            .any(|s| s.eq_ignore_ascii_case(service_name))
    }

    /// Verifica se uma chave de registro é segura para modificação
    pub fn is_safe_registry_key(&self, key: &str) -> bool {
        !self.critical_registry_keys
            .iter()
            .any(|critical| key.starts_with(critical))
    }

    /// Valida se uma operação é segura
    pub fn validate_operation(&self, operation_type: &str, target: &str) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut safety_score = 100u8;

        match operation_type {
            "service_disable" => {
                if self.is_critical_service(target) {
                    errors.push(ValidationError {
                        code: "CRITICAL_SERVICE".to_string(),
                        message: format!("Cannot disable critical service: {}", target),
                        severity: "CRITICAL".to_string(),
                    });
                    safety_score = 0;
                }
            }
            "registry_modify" => {
                if !self.is_safe_registry_key(target) {
                    warnings.push(ValidationWarning {
                        code: "CRITICAL_REGISTRY".to_string(),
                        message: format!("Modifying critical registry key: {}", target),
                        recommendation: "Backup will be created automatically".to_string(),
                    });
                    safety_score = safety_score.saturating_sub(30);
                }
            }
            _ => {}
        }

        if errors.is_empty() {
            ValidationResult {
                valid: true,
                errors,
                warnings,
                safety_score,
            }
        } else {
            ValidationResult::failure(errors)
        }
    }
}

impl Default for SafetyValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_critical_service_detection() {
        let validator = SafetyValidator::new();
        assert!(validator.is_critical_service("wuauserv"));
        assert!(validator.is_critical_service("EventLog"));
        assert!(!validator.is_critical_service("SomeRandomService"));
    }

    #[test]
    fn test_service_validation() {
        let validator = SafetyValidator::new();
        
        let result = validator.validate_operation("service_disable", "wuauserv");
        assert!(!result.valid);
        assert!(!result.errors.is_empty());
        
        let result = validator.validate_operation("service_disable", "SafeService");
        assert!(result.valid);
    }
}
