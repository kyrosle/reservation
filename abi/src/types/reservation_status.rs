use crate::ReservationStatus;

impl std::fmt::Display for ReservationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReservationStatus::Pending => write!(f, "pending"),
            ReservationStatus::Blocked => write!(f, "blocked"),
            ReservationStatus::Confirmed => write!(f, "confirmed"),
            ReservationStatus::Unknown => write!(f, "unknown"),
        }
    }
}
