use chrono::{DateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

use crate::infrastructure::database::schema::tenants;

#[derive(Debug, Clone, Queryable, Selectable, Identifiable)]
#[diesel(table_name = tenants)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct TenantModel {
    pub id: Uuid,
    pub name: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = tenants)]
pub struct NewTenantModel {
    pub name: String,
}
