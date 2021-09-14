use twilight_model::{
    id::UserId,
    user::{CurrentUser, User},
};

pub trait UserExt {
    fn tag(&self) -> String;
}

impl UserExt for User {
    fn tag(&self) -> String {
        format!("{}#{:04}", self.name, self.discriminator)
    }
}

impl UserExt for CurrentUser {
    fn tag(&self) -> String {
        format!("{}#{:04}", self.name, self.discriminator)
    }
}

pub trait ShallowUser {
    fn id(&self) -> UserId;
    fn name(&self) -> String;
}

impl ShallowUser for &CurrentUser {
    fn id(&self) -> UserId {
        self.id
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

impl ShallowUser for &User {
    fn id(&self) -> UserId {
        self.id
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}
