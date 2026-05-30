// Inline registrar module

#[derive(Debug)]
pub struct ServiceRegistry {
    pub services: std::collections::HashMap<String, Vec<String>>,
}

impl ServiceRegistry {
    pub fn new() -> Self {
        Self { services: std::collections::HashMap::new() }
    }
    
    pub fn register(&mut self, identity: &str, service_id: &str) {
        self.services
            .entry(identity.to_string())
            .or_insert_with(Vec::new)
            .push(service_id.to_string());
    }
    
    pub fn get_services(&self, identity: &str) -> Option<&Vec<String>> {
        self.services.get(identity)
    }
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub fn create_registry() -> ServiceRegistry {
    ServiceRegistry::new()
}
