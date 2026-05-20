use crate::services::identity::{
    entities::{IdentityAssociation, IdentityHashAlgorithm, LookupPepper},
    errors::IdentityServiceError,
};

pub struct LookupAssociationsCommand {
    pub addresses: Vec<String>,
    pub algorithm: IdentityHashAlgorithm,
    pub pepper: LookupPepper,
}

pub trait IdentityAssociationLookupRepository: Send + Sync {
    fn lookup_associations(
        &self,
        command: &LookupAssociationsCommand,
    ) -> Result<Vec<IdentityAssociation>, IdentityServiceError>;

    fn lookup_pepper(&self) -> Result<LookupPepper, IdentityServiceError>;

    fn supported_hash_algorithms(&self)
    -> Result<Vec<IdentityHashAlgorithm>, IdentityServiceError>;
}

pub struct AssociationLookupService<Repository>
where
    Repository: IdentityAssociationLookupRepository,
{
    association_lookup_repository: Repository,
}

impl<Repository> AssociationLookupService<Repository>
where
    Repository: IdentityAssociationLookupRepository,
{
    #[must_use]
    pub const fn new(association_lookup_repository: Repository) -> Self {
        Self {
            association_lookup_repository,
        }
    }

    pub fn lookup_associations(
        &self,
        command: &LookupAssociationsCommand,
    ) -> Result<Vec<IdentityAssociation>, IdentityServiceError> {
        self.association_lookup_repository
            .lookup_associations(command)
    }

    pub fn lookup_pepper(&self) -> Result<LookupPepper, IdentityServiceError> {
        self.association_lookup_repository.lookup_pepper()
    }

    pub fn supported_hash_algorithms(
        &self,
    ) -> Result<Vec<IdentityHashAlgorithm>, IdentityServiceError> {
        self.association_lookup_repository
            .supported_hash_algorithms()
    }
}
