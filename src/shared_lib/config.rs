pub const PUBLIC_ROOM_ID: &str = "7e40f106-3e7d-498a-94cc-5fa7f62cfce6";
pub const PUBLIC_ROOM_NAME: &str = "public room";

pub const USERNAME_RE_PATTERN: &str = "^[A-Za-z][A-Za-z0-9_]{7,29}$";
pub const PASSWORD_RE_PATTERN: &str = r"^[A-Za-z\d!@#$%^&*()_+]{8,32}$";

pub const USERNAME_ERROR_MSG: &str= "Username must start with a letter, not contain special characters ouside of \"_\" and have length between 7 to 29";
pub const PASSWORD_ERROR_MSG: &str= "Password must contain at least one lowercase and uppercase letter, digit and have length between 8 to 32";

pub const SERVER_ADDR: &str = "localhost:11111";
