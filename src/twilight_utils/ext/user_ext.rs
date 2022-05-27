use twilight_model::{
    id::{marker::UserMarker, Id},
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
    fn id(&self) -> Id<UserMarker>;
    fn name(&self) -> String;
}

impl ShallowUser for &CurrentUser {
    fn id(&self) -> Id<UserMarker> {
        self.id
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}

impl ShallowUser for &User {
    fn id(&self) -> Id<UserMarker> {
        self.id
    }

    fn name(&self) -> String {
        self.name.clone()
    }
}
