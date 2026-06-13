use axum::{routing::get, Json, Router};
use serde::Serialize;

use crate::state::AppState;

#[derive(Serialize)]
struct ScopeResponse {
    module: &'static str,
    phase: &'static str,
    implemented_now: Vec<&'static str>,
    api_contract: SocialApiContract,
    database_contract: DatabaseContract,
    next_backend_steps: Vec<&'static str>,
}

#[derive(Serialize)]
struct SocialApiContract {
    feed: Vec<RouteContract>,
    posts: Vec<RouteContract>,
    comments: Vec<RouteContract>,
    reactions: Vec<RouteContract>,
    profiles: Vec<RouteContract>,
    moderation: Vec<RouteContract>,
}

#[derive(Serialize)]
struct DatabaseContract {
    source_of_truth: &'static str,
    core_tables: Vec<&'static str>,
    media_join_strategy: &'static str,
    privacy_rule: &'static str,
}

#[derive(Serialize)]
struct RouteContract {
    method: &'static str,
    path: &'static str,
    auth: &'static str,
    purpose: &'static str,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/scope", get(scope))
}

async fn scope() -> Json<ScopeResponse> {
    Json(ScopeResponse {
        module: module_path!(),
        phase: "public-social-schema-and-api-contract",
        implemented_now: vec![
            "public-social-database-contract",
            "feed-api-contract",
            "media-link-contract",
            "public-profile-contract",
            "moderation-contract",
        ],
        api_contract: SocialApiContract {
            feed: vec![RouteContract {
                method: "GET",
                path: "/v1/social/feed",
                auth: "optional-session",
                purpose: "List latest visible community posts with author profile, stats, viewer state, and media attachments.",
            }],
            posts: vec![
                RouteContract {
                    method: "POST",
                    path: "/v1/social/posts",
                    auth: "required-session",
                    purpose: "Create a published social post and attach uploaded media assets through media_links.",
                },
                RouteContract {
                    method: "GET",
                    path: "/v1/social/posts/:post_id",
                    auth: "optional-session",
                    purpose: "Read one visible post with comments, author profile, media attachments, stats, and viewer state.",
                },
                RouteContract {
                    method: "POST",
                    path: "/v1/social/posts/:post_id/hide",
                    auth: "required-session",
                    purpose: "Hide a post for the current viewer without moderating or deleting it globally.",
                },
            ],
            comments: vec![RouteContract {
                method: "POST",
                path: "/v1/social/posts/:post_id/comments",
                auth: "required-session",
                purpose: "Create a visible comment for a published post.",
            }],
            reactions: vec![
                RouteContract {
                    method: "POST",
                    path: "/v1/social/posts/:post_id/reactions",
                    auth: "required-session",
                    purpose: "Add or replace the viewer reaction for a post.",
                },
                RouteContract {
                    method: "DELETE",
                    path: "/v1/social/posts/:post_id/reactions/:kind",
                    auth: "required-session",
                    purpose: "Remove the viewer reaction from a post.",
                },
                RouteContract {
                    method: "POST",
                    path: "/v1/social/comments/:comment_id/reactions",
                    auth: "required-session",
                    purpose: "Add or replace the viewer reaction for a comment.",
                },
                RouteContract {
                    method: "DELETE",
                    path: "/v1/social/comments/:comment_id/reactions/:kind",
                    auth: "required-session",
                    purpose: "Remove the viewer reaction from a comment.",
                },
            ],
            profiles: vec![
                RouteContract {
                    method: "GET",
                    path: "/v1/social/profiles/:user_id",
                    auth: "optional-session",
                    purpose: "Read a public/community-safe profile card for social authors without exposing email.",
                },
                RouteContract {
                    method: "POST",
                    path: "/v1/social/profiles/:user_id/follow",
                    auth: "required-session",
                    purpose: "Follow a visible community profile.",
                },
                RouteContract {
                    method: "DELETE",
                    path: "/v1/social/profiles/:user_id/follow",
                    auth: "required-session",
                    purpose: "Unfollow a community profile.",
                },
            ],
            moderation: vec![
                RouteContract {
                    method: "POST",
                    path: "/v1/social/posts/:post_id/report",
                    auth: "required-session",
                    purpose: "Report a post for admin moderation review.",
                },
                RouteContract {
                    method: "POST",
                    path: "/v1/social/comments/:comment_id/report",
                    auth: "required-session",
                    purpose: "Report a comment for admin moderation review.",
                },
            ],
        },
        database_contract: DatabaseContract {
            source_of_truth: "migrations/0069_public_social_schema.sql",
            core_tables: vec![
                "social_posts",
                "social_comments",
                "social_reactions",
                "social_follows",
                "social_post_hides",
                "social_reports",
                "social_moderation_actions",
                "media_links",
                "profiles",
            ],
            media_join_strategy: "Social posts and comments use media_links with entity_type values such as social_post and social_comment. Media ownership remains in media_assets.",
            privacy_rule: "Public social responses must never expose user email. Author identity comes from profiles only.",
        },
        next_backend_steps: vec![
            "implement social repository queries",
            "implement authenticated create/comment/reaction/report endpoints",
            "hydrate feed with profile cards and media attachments",
            "add admin moderation read/actions",
            "connect frontend social gateway to backend by default",
        ],
    })
}
