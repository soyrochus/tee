use crate::domain::principal::Principal;

// Placeholder for authorization.
// Replace with real policy checks using the Principal context.
pub fn can_create_task(_principal: &Principal) -> bool {
    true
}

pub fn can_start_task(_principal: &Principal) -> bool {
    true
}

pub fn can_complete_task(_principal: &Principal) -> bool {
    true
}

pub fn can_update_task_details(_principal: &Principal) -> bool {
    true
}

pub fn can_delete_task(_principal: &Principal) -> bool {
    true
}

pub fn can_view_tasks(_principal: &Principal) -> bool {
    true
}

pub fn can_view_task(_principal: &Principal) -> bool {
    true
}
