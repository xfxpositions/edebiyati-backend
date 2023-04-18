mod common;
mod permissions;
mod post;
mod tag;
mod user;

#[no_mangle]
pub static DEFAULT_POST_IMAGE: &'static str = "https://www.eska.org.tr/wp-content/uploads/2021/01/k2-winter.jpg";


pub use common::Common;
pub use permissions::Permission;
pub use post::Post;
pub use tag::Tag;
pub use user::User;
pub use post::Comment;
pub use post::Content;
pub use post::PostStatus;

