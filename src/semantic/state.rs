#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OwnershipState {
    Active,
    PartiallyMoved { fields: Vec<String> },
    Moved,
    Dropped,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AccessState {
    Mutable,
    Frozen { borrow_ids: Vec<usize> },
}

impl OwnershipState {
    pub fn is_live(&self) -> bool {
        matches!(self, Self::Active | Self::PartiallyMoved { .. })
    }

    pub fn can_be_read(&self) -> bool {
        self.is_live()
    }

    pub fn can_be_cleaned(&self) -> bool {
        self.is_live()
    }
}

impl AccessState {
    pub fn is_frozen(&self) -> bool {
        matches!(self, Self::Frozen { .. })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceState {
    pub ownership: OwnershipState,
    pub access: AccessState,
}

impl ResourceState {
    pub fn active() -> Self {
        Self { ownership: OwnershipState::Active, access: AccessState::Mutable }
    }

    pub fn freeze(&mut self, borrow_id: usize) -> bool {
        if !matches!(self.ownership, OwnershipState::Active)
            || !matches!(self.access, AccessState::Mutable)
        {
            return false;
        }
        self.access = AccessState::Frozen { borrow_ids: vec![borrow_id] };
        true
    }

    pub fn thaw(&mut self) -> bool {
        if !matches!(self.ownership, OwnershipState::Active | OwnershipState::PartiallyMoved { .. })
            || !self.access.is_frozen()
        {
            return false;
        }
        self.access = AccessState::Mutable;
        true
    }

    pub fn move_owner(&mut self) -> bool {
        if !matches!(self.ownership, OwnershipState::Active)
            || !matches!(self.access, AccessState::Mutable)
        {
            return false;
        }
        self.ownership = OwnershipState::Moved;
        true
    }

    pub fn partial_move(&mut self, field: impl Into<String>) -> bool {
        if !matches!(self.ownership, OwnershipState::Active)
            || !matches!(self.access, AccessState::Mutable)
        {
            return false;
        }
        self.ownership = OwnershipState::PartiallyMoved { fields: vec![field.into()] };
        true
    }

    pub fn drop_owner(&mut self) -> bool {
        if !matches!(self.ownership, OwnershipState::Active | OwnershipState::PartiallyMoved { .. })
            || !matches!(self.access, AccessState::Mutable)
        {
            return false;
        }
        self.ownership = OwnershipState::Dropped;
        true
    }
}
