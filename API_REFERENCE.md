# Jungle Backend — API Reference

Base URL: `http://localhost:8080`
Auth: `Authorization: Bearer <access_token>` (except public routes)

> Last regenerated: 2026-04-24 (post Wave-D migration). Includes Web Push,
> AI chat suggestions, Live VOD, image rotate, GIF proxy, OAuth verify,
> backups/restore and DLQ admin endpoints.

---

## Auth Service

### Core Auth
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/v1/auth/register` | No | Register new user |
| POST | `/v1/auth/login` | No | Login with email/phone + password |
| POST | `/v1/auth/refresh` | No | Refresh access token (uses httpOnly cookie) |
| POST | `/v1/auth/logout` | Yes | Logout current session |
| GET | `/v1/auth/me` | Yes | Get current authenticated user |

### Password
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/v1/auth/forgot-password` | No | Send password reset email |
| POST | `/v1/auth/reset-password` | No | Reset password with token |
| PUT | `/v1/auth/password` | Yes | Change password |

### Verification
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/v1/auth/verify-email` | No | Verify email with code |
| POST | `/v1/auth/verify-phone` | No | Verify phone with code |
| POST | `/v1/auth/resend-code` | No | Resend verification code |

### Two-Factor Auth
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/v1/auth/2fa/setup` | Yes | Setup 2FA (returns QR/secret) |
| POST | `/v1/auth/2fa/enable` | Yes | Enable 2FA after setup |
| POST | `/v1/auth/2fa/verify` | No | Verify 2FA code during login |
| POST | `/v1/auth/2fa/disable` | Yes | Disable 2FA |
| GET | `/v1/auth/2fa/backup-codes` | Yes | List backup codes |
| POST | `/v1/auth/2fa/backup-codes/regenerate` | Yes | Regenerate backup codes |

### Social Login
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/v1/auth/social/login` | No | Social login (body: `{provider, token}`) — supports: google, facebook, twitter, apple, linkedin, github, microsoft, discord, tiktok, instagram, vkontakte, qq, wechat, mailru, okru, wordpress |

### Sessions
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/auth/sessions` | Yes | List all active sessions |
| DELETE | `/v1/auth/sessions/{id}` | Yes | Revoke a specific session |
| POST | `/v1/auth/sessions/revoke-all` | Yes | Revoke all sessions |
| POST | `/v1/auth/switch-account` | Yes | Switch to another account |

### OAuth Developer Portal
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/oauth/apps` | Yes | List my OAuth apps |
| POST | `/v1/oauth/apps` | Yes | Create OAuth app |
| GET | `/v1/oauth/apps/{id}` | Yes | Get OAuth app details |
| PUT | `/v1/oauth/apps/{id}` | Yes | Update OAuth app |
| DELETE | `/v1/oauth/apps/{id}` | Yes | Delete OAuth app |
| GET | `/v1/oauth/apps/{id}/permissions` | Yes | Get app permissions |
| POST | `/v1/oauth/authorize` | Yes | Authorize OAuth app |
| POST | `/v1/oauth/token` | No | Exchange code for token |
| POST | `/v1/oauth/revoke` | Yes | Revoke OAuth token |

### Public
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/translations/{lang}` | No | Get translations for a language |
| GET | `/v1/config/public` | No | Get public site config |
| GET | `/v1/site-settings` | No | Get site settings |
| GET | `/v1/auth/check` | No | Check username/email availability |
| GET | `/v1/auth/is-active` | No | Check if site is active |


---

## User Service

### Profile
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/users/me` | Yes | Get my profile |
| PUT | `/v1/users/me` | Yes | Update my profile |
| DELETE | `/v1/users/me` | Yes | Delete my account |
| PUT | `/v1/users/me/avatar` | Yes | Update avatar |
| PUT | `/v1/users/me/cover` | Yes | Update cover photo |
| GET | `/v1/users/{username}` | Yes | Get user profile by username |
| PUT | `/v1/users/me/social-links` | Yes | Update social links |
| GET | `/v1/users/{username}/social-links` | No | Get user social links |

### Search & Discovery
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/users/search` | Yes | Search users |
| GET | `/v1/users/suggestions` | Yes | Get follow suggestions |
| GET | `/v1/users/pro-users` | Yes | List pro users |
| GET | `/v1/mentions` | Yes | Mention autocomplete (`?q=`) |
| GET | `/v1/users/nearby` | Yes | Nearby users (requires location) |
| GET | `/v1/users/birthdays` | Yes | Friends with birthdays today |
| POST | `/v1/users/batch` | Yes | Batch fetch users by IDs |
| GET | `/v1/users/by-phone` | Yes | Get user by phone number |

### Social Graph
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/v1/social/follow/{user_id}` | Yes | Follow user |
| DELETE | `/v1/social/follow/{user_id}` | Yes | Unfollow user |
| GET | `/v1/users/{username}/followers` | Yes | Get followers list |
| GET | `/v1/users/{username}/following` | Yes | Get following list |
| GET | `/v1/social/follow-requests` | Yes | List pending follow requests |
| POST | `/v1/social/follow-requests/{id}/accept` | Yes | Accept follow request |
| POST | `/v1/social/follow-requests/{id}/reject` | Yes | Reject follow request |
| GET | `/v1/social/blocked` | Yes | List blocked users |
| POST | `/v1/social/block/{user_id}` | Yes | Block user |
| DELETE | `/v1/social/block/{user_id}` | Yes | Unblock user |
| POST | `/v1/social/poke/{user_id}` | Yes | Poke user |
| POST | `/v1/social/mute/{user_id}` | Yes | Mute user |
| DELETE | `/v1/social/mute/{user_id}` | Yes | Unmute user |
| POST | `/v1/social/family/{user_id}` | Yes | Send family request |
| PUT | `/v1/social/family/{id}` | Yes | Respond to family request |
| POST | `/v1/social/stop-notify/{user_id}` | Yes | Stop post notifications from user |

### Professional (LinkedIn Mode)
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/users/{user_id}/experience` | Yes | Get work experience |
| POST | `/v1/users/me/experience` | Yes | Add work experience |
| DELETE | `/v1/users/me/experience/{id}` | Yes | Delete work experience |
| GET | `/v1/users/{user_id}/certifications` | Yes | Get certifications |
| POST | `/v1/users/me/certifications` | Yes | Add certification |
| DELETE | `/v1/users/me/certifications/{id}` | Yes | Delete certification |
| GET | `/v1/users/{user_id}/projects` | Yes | Get projects |
| POST | `/v1/users/me/projects` | Yes | Add project |
| DELETE | `/v1/users/me/projects/{id}` | Yes | Delete project |
| GET | `/v1/users/{user_id}/mutual-friends` | Yes | Get mutual friends |
| GET | `/v1/users/{username}/skills` | Yes | Get user skills |
| GET | `/v1/skills/search` | Yes | Search skills autocomplete |
| POST | `/v1/users/me/skills` | Yes | Add skill |
| DELETE | `/v1/users/me/skills/{id}` | Yes | Remove skill |
| POST | `/v1/users/me/open-to-work` | Yes | Set open to work |
| DELETE | `/v1/users/me/open-to-work` | Yes | Unset open to work |
| POST | `/v1/users/me/providing-service` | Yes | Set providing service |
| DELETE | `/v1/users/me/providing-service` | Yes | Unset providing service |

### User Content
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/users/{username}/posts` | Yes | Get user posts |
| GET | `/v1/users/{username}/photos` | Yes | Get user photos |
| GET | `/v1/users/{username}/videos` | Yes | Get user videos |
| GET | `/v1/users/{user_id}/common` | Yes | Common things with user |

### Settings
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/users/me/privacy` | Yes | Get privacy settings |
| PUT | `/v1/users/me/privacy` | Yes | Update privacy settings |
| GET | `/v1/users/me/notification-settings` | Yes | Get notification settings |
| PUT | `/v1/users/me/notification-settings` | Yes | Update notification settings |
| GET | `/v1/users/me/invite-code` | Yes | Get my invite code |
| GET | `/v1/users/me/fields` | Yes | Get my custom field values |
| PUT | `/v1/users/me/fields` | Yes | Update my custom field values |
| GET | `/v1/users/{user_id}/fields` | Yes | Get user custom field values |

### Addresses
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/users/me/addresses` | Yes | List my addresses |
| POST | `/v1/users/me/addresses` | Yes | Create address |
| GET | `/v1/users/me/addresses/{id}` | Yes | Get address |
| PUT | `/v1/users/me/addresses/{id}` | Yes | Update address |
| DELETE | `/v1/users/me/addresses/{id}` | Yes | Delete address |

### Misc
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/v1/users/me/avatar/reset` | Yes | Reset avatar to default |
| POST | `/v1/users/me/download-info` | Yes | GDPR data export |
| PUT | `/v1/users/me/location` | Yes | Update location |
| PUT | `/v1/users/me/lastseen` | Yes | Update last seen |
| GET | `/v1/users/me/referrals` | Yes | My referrals |
| GET | `/v1/users/me/inviters` | Yes | My inviters |
| POST | `/v1/users/me/onboarding/skip` | Yes | Skip onboarding step |
| POST | `/v1/search/register` | Yes | Register recent search |
| POST | `/v1/contact` | No | Contact form |
| POST | `/v1/general` | Yes | General data batch fetch (mobile startup) |
| POST | `/v1/reports` | Yes | Create a report |
| POST | `/v1/points/admob` | Yes | Record AdMob points |
| GET | `/v1/activities` | Yes | List my activities |


---

## Post Service

### Feed
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/feed` | Yes | Get personalized news feed (cursor pagination) |
| GET | `/v1/feed/explore` | Yes | Explore feed (trending/public posts) |
| GET | `/v1/memories` | Yes | "On this day" memories |

### Posts CRUD
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/v1/posts` | Yes | Create post |
| GET | `/v1/posts/{id}` | Yes | Get post by ID |
| PUT | `/v1/posts/{id}` | Yes | Update post |
| DELETE | `/v1/posts/{id}` | Yes | Delete post |
| POST | `/v1/posts/{id}/save` | Yes | Save post |
| DELETE | `/v1/posts/{id}/save` | Yes | Unsave post |
| POST | `/v1/posts/{id}/hide` | Yes | Hide post from feed |
| POST | `/v1/posts/{id}/share` | Yes | Share/repost |
| POST | `/v1/posts/{id}/pin` | Yes | Pin post to profile |
| DELETE | `/v1/posts/{id}/pin` | Yes | Unpin post |
| POST | `/v1/posts/{id}/boost` | Yes | Boost post (Pro) |
| POST | `/v1/posts/{id}/report` | Yes | Report post |
| POST | `/v1/posts/{id}/poll/vote` | Yes | Vote on poll |
| GET | `/v1/posts/colored-templates` | No | List colored post backgrounds |
| GET | `/v1/posts/reaction-types` | No | List reaction types |
| GET | `/v1/posts/most-liked` | Yes | Most liked posts |
| GET | `/v1/posts/most-watched` | Yes | Most watched posts |
| GET | `/v1/boosted/posts` | Yes | My boosted posts |

### Reactions
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/v1/posts/{id}/react` | Yes | React to post (body: `{reaction_type}`) |
| DELETE | `/v1/posts/{id}/react` | Yes | Remove reaction from post |
| POST | `/v1/comments/{id}/react` | Yes | React to comment |

### Comments
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/posts/{id}/comments` | Yes | Get post comments |
| POST | `/v1/posts/{id}/comments` | Yes | Create comment |
| PUT | `/v1/comments/{id}` | Yes | Update comment |
| DELETE | `/v1/comments/{id}` | Yes | Delete comment |
| GET | `/v1/comments/{id}/replies` | Yes | Get comment replies |
| POST | `/v1/comments/{id}/replies` | Yes | Create reply |

### Reels
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/reels` | Yes | Get reels feed |
| POST | `/v1/reels` | Yes | Create reel |
| GET | `/v1/reels/{id}` | Yes | Get reel |
| DELETE | `/v1/reels/{id}` | Yes | Delete reel |
| POST | `/v1/reels/{id}/view` | Yes | Record reel view |
| POST | `/v1/reels/{id}/react` | Yes | React to reel |
| GET | `/v1/reels/{id}/comments` | Yes | Get reel comments |
| POST | `/v1/reels/{id}/comments` | Yes | Add reel comment |

### Search
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/search` | Yes | Global search (`?q=&type=user\|post\|page\|group\|hashtag\|blog\|product`) |
| GET | `/v1/search/recent` | Yes | List recent searches |
| POST | `/v1/search/recent` | Yes | Save recent search |
| DELETE | `/v1/search/recent` | Yes | Clear recent searches |

### Hashtags
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/hashtags/trending` | Yes | Trending hashtags |
| GET | `/v1/hashtags/search` | Yes | Search hashtags (`?q=`) |
| GET | `/v1/hashtags/{tag}/posts` | Yes | Posts by hashtag |

### Albums
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/v1/albums` | Yes | Create album |
| GET | `/v1/users/{user_id}/albums` | Yes | List user albums |
| GET | `/v1/albums/{id}/images` | Yes | List album images |
| POST | `/v1/albums/{id}/images` | Yes | Add images to album |
| DELETE | `/v1/albums/{album_id}/images/{image_id}` | Yes | Delete image from album |

### User Ads
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/v1/ads` | Yes | Create ad |
| GET | `/v1/ads/my` | Yes | My ads |
| GET | `/v1/ads/{id}/stats` | Yes | Ad statistics |
| PUT | `/v1/ads/{id}` | Yes | Update ad |
| DELETE | `/v1/ads/{id}` | Yes | Cancel ad |
| POST | `/v1/ads/{id}/click` | No | Record ad click |
| POST | `/v1/ads/{id}/view` | No | Record ad view |
| GET | `/v1/ads/estimated-audience` | Yes | Get estimated audience |

### Live Streaming
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/v1/live/start` | Yes | Start live stream |
| POST | `/v1/live/stop` | Yes | Stop live stream |
| GET | `/v1/live/active` | Yes | Active live streams |
| GET | `/v1/live/friends` | Yes | Friends currently live |
| POST | `/v1/live/{id}/comment` | Yes | Comment on live stream |
| POST | `/v1/live/{id}/react` | Yes | React to live stream |
| GET | `/v1/live/{id}/vod` | Yes | Get VOD playback metadata for an ended stream (`vod_url`, `vod_thumbnail`, `vod_duration_seconds`, `vod_ready_at`) |
| PATCH | `/v1/live/{id}/vod` | Admin/Worker | Publish VOD metadata after the transcoder finishes (used by `live-vod-transcoder`) |


---

## Messaging Service

### Conversations
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/conversations` | Yes | List conversations |
| POST | `/v1/conversations` | Yes | Create direct conversation |
| POST | `/v1/conversations/group` | Yes | Create group conversation |
| GET | `/v1/conversations/pinned` | Yes | List pinned conversations |
| GET | `/v1/conversations/archived` | Yes | List archived conversations |
| GET | `/v1/conversations/{id}` | Yes | Get conversation |
| DELETE | `/v1/conversations/{id}` | Yes | Delete conversation |
| PUT | `/v1/conversations/{id}/color` | Yes | Update conversation color |
| POST | `/v1/conversations/{id}/pin` | Yes | Pin conversation |
| DELETE | `/v1/conversations/{id}/pin` | Yes | Unpin conversation |
| POST | `/v1/conversations/{id}/archive` | Yes | Archive conversation |
| DELETE | `/v1/conversations/{id}/archive` | Yes | Unarchive conversation |
| POST | `/v1/conversations/{id}/read` | Yes | Mark conversation as read |
| POST | `/v1/conversations/mark-all-read` | Yes | Mark all conversations as read |
| PUT | `/v1/conversations/group/{id}` | Yes | Update group conversation info |

### Messages
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/conversations/{id}/messages` | Yes | List messages (cursor pagination) |
| POST | `/v1/conversations/{id}/messages` | Yes | Send message |
| POST | `/v1/conversations/{id}/typing` | Yes | Send typing indicator |
| DELETE | `/v1/messages/{id}` | Yes | Delete message |
| POST | `/v1/messages/{id}/favorite` | Yes | Toggle message favorite |
| POST | `/v1/messages/{id}/pin` | Yes | Pin message |
| DELETE | `/v1/messages/{id}/pin` | Yes | Unpin message |
| POST | `/v1/messages/{id}/forward` | Yes | Forward message |
| POST | `/v1/messages/{id}/react` | Yes | React to message |
| POST | `/v1/messages/{id}/listened` | Yes | Mark audio message as listened |

### Broadcasts
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/broadcasts` | Yes | List broadcasts |
| POST | `/v1/broadcasts` | Yes | Create broadcast |
| PUT | `/v1/broadcasts/{id}` | Yes | Update broadcast |
| DELETE | `/v1/broadcasts/{id}` | Yes | Delete broadcast |
| GET | `/v1/broadcasts/{id}/members` | Yes | List broadcast members |
| POST | `/v1/broadcasts/{id}/members` | Yes | Add members |
| DELETE | `/v1/broadcasts/{id}/members/{user_id}` | Yes | Remove member |
| POST | `/v1/broadcasts/{id}/send` | Yes | Send broadcast message |

### Calls
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/calls` | Yes | List call history |
| POST | `/v1/calls` | Yes | Initiate call |
| POST | `/v1/calls/agora-token` | Yes | Generate Agora token |
| GET | `/v1/calls/{id}` | Yes | Get call details |
| PUT | `/v1/calls/{id}/status` | Yes | Update call status (accept/reject/end) |

---

## Notification Service

### Notifications
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/notifications` | Yes | List notifications (cursor pagination) |
| GET | `/v1/notifications/unread-count` | Yes | Get unread count |
| POST | `/v1/notifications/read-all` | Yes | Mark all as read |
| POST | `/v1/notifications/{id}/read` | Yes | Mark notification as read |
| DELETE | `/v1/notifications/{id}` | Yes | Delete notification |
| DELETE | `/v1/notifications/clear` | Yes | Clear all notifications |

### Preferences
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/notifications/preferences` | Yes | Get notification preferences |
| PUT | `/v1/notifications/preferences` | Yes | Update notification preferences |

### Push Tokens (FCM/APNs)
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/v1/notifications/push-tokens` | Yes | Register push token (FCM/APNs) |
| GET | `/v1/notifications/push-tokens` | Yes | List my push tokens |
| DELETE | `/v1/notifications/push-tokens/{token}` | Yes | Unregister push token |

### VAPID Web Push (browsers / PWAs)
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/notifications/web-push/public-key` | No | Get VAPID public key for `pushManager.subscribe` |
| POST | `/v1/notifications/web-push/subscribe` | Yes | Register a Web Push subscription (`{endpoint, p256dh, auth, user_agent?}`) |
| POST | `/v1/notifications/web-push/unsubscribe` | Yes | Remove a subscription by `endpoint` |
| GET | `/v1/notifications/web-push/subscriptions` | Yes | List my registered Web Push subscriptions |
| DELETE | `/v1/notifications/web-push/subscriptions/{id}` | Yes | Delete a single Web Push subscription |

### Announcements
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/announcements` | Yes | List active announcements |
| POST | `/v1/announcements/{id}/dismiss` | Yes | Dismiss announcement |

### Newsletter
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/v1/newsletter/subscribe` | No | Subscribe to newsletter |
| POST | `/v1/newsletter/unsubscribe` | No | Unsubscribe from newsletter |


---

## Group & Page Service

### Pages
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/v1/pages` | Yes | Create page |
| GET | `/v1/pages/categories` | No | List page categories |
| GET | `/v1/pages/search` | Yes | Search pages |
| GET | `/v1/pages/suggested` | Yes | Suggested pages |
| GET | `/v1/pages/my` | Yes | My pages |
| GET | `/v1/pages/liked` | Yes | Pages I liked |
| GET | `/v1/pages/check-name` | Yes | Check page name availability |
| GET | `/v1/pages/{slug}` | Yes | Get page by slug |
| PUT | `/v1/pages/{id}` | Yes | Update page |
| DELETE | `/v1/pages/{id}` | Yes | Delete page |
| POST | `/v1/pages/{id}/like` | Yes | Like page |
| DELETE | `/v1/pages/{id}/like` | Yes | Unlike page |
| POST | `/v1/pages/{id}/rate` | Yes | Rate page (1-5) |
| GET | `/v1/pages/{id}/likes` | Yes | List page likers |
| GET | `/v1/pages/{id}/ratings` | Yes | List page ratings |
| GET | `/v1/pages/{id}/admins` | Yes | List page admins |
| POST | `/v1/pages/{id}/admins` | Yes | Add page admin |
| DELETE | `/v1/pages/{id}/admins/{user_id}` | Yes | Remove page admin |
| GET | `/v1/pages/{id}/posts` | Yes | Get page posts |
| POST | `/v1/pages/{id}/invite` | Yes | Invite users to like page |
| PUT | `/v1/pages/{id}/avatar` | Yes | Update page avatar |
| PUT | `/v1/pages/{id}/cover` | Yes | Update page cover |
| POST | `/v1/pages/{id}/boost` | Yes | Boost page (Pro) |
| POST | `/v1/pages/{id}/verify` | Yes | Request page verification |
| GET | `/v1/pages/{id}/non-likes` | Yes | Users who haven't liked |
| GET | `/v1/boosted/pages` | Yes | My boosted pages |

### Groups
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/v1/groups` | Yes | Create group |
| GET | `/v1/groups/categories` | No | List group categories |
| GET | `/v1/groups/search` | Yes | Search groups |
| GET | `/v1/groups/suggested` | Yes | Suggested groups |
| GET | `/v1/groups/my` | Yes | My groups (created) |
| GET | `/v1/groups/joined` | Yes | Groups I joined |
| GET | `/v1/groups/check-name` | Yes | Check group name availability |
| GET | `/v1/groups/{slug}` | Yes | Get group by slug |
| PUT | `/v1/groups/{id}` | Yes | Update group |
| DELETE | `/v1/groups/{id}` | Yes | Delete group |
| POST | `/v1/groups/{id}/join` | Yes | Join group |
| DELETE | `/v1/groups/{id}/join` | Yes | Leave group |
| GET | `/v1/groups/{id}/members` | Yes | List members |
| DELETE | `/v1/groups/{id}/members/{uid}` | Yes | Kick member |
| POST | `/v1/groups/{id}/members/{uid}/role` | Yes | Change member role |
| GET | `/v1/groups/{id}/join-requests` | Yes | List join requests |
| POST | `/v1/groups/{id}/join-requests/{rid}/accept` | Yes | Accept join request |
| POST | `/v1/groups/{id}/join-requests/{rid}/reject` | Yes | Reject join request |
| GET | `/v1/groups/{id}/posts` | Yes | Get group posts |
| POST | `/v1/groups/{id}/invite` | Yes | Invite users to group |
| PUT | `/v1/groups/{id}/avatar` | Yes | Update group avatar |
| PUT | `/v1/groups/{id}/cover` | Yes | Update group cover |
| GET | `/v1/groups/{id}/non-members` | Yes | Users who aren't members |

### Events
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/v1/events` | Yes | Create event |
| GET | `/v1/events/upcoming` | Yes | Upcoming events |
| GET | `/v1/events/my` | Yes | My events |
| GET | `/v1/events/attending` | Yes | Events I'm attending |
| GET | `/v1/events/{id}` | Yes | Get event |
| PUT | `/v1/events/{id}` | Yes | Update event |
| DELETE | `/v1/events/{id}` | Yes | Delete event |
| POST | `/v1/events/{id}/respond` | Yes | RSVP (body: `{response: going\|interested\|not_going}`) |
| GET | `/v1/events/{id}/going` | Yes | List going attendees |
| GET | `/v1/events/{id}/interested` | Yes | List interested attendees |
| POST | `/v1/events/{id}/invite` | Yes | Invite users to event |
| GET | `/v1/events/{id}/posts` | Yes | Get event posts |
| PUT | `/v1/events/{id}/cover` | Yes | Update event cover |


---

## Content Service

### Blogs
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/blogs` | Yes | List blogs |
| POST | `/v1/blogs` | Yes | Create blog |
| GET | `/v1/blogs/search` | Yes | Search blogs |
| GET | `/v1/blogs/my` | Yes | My blogs |
| GET | `/v1/blogs/categories` | No | List blog categories |
| GET | `/v1/blogs/category/{id}` | Yes | Blogs by category |
| GET | `/v1/blogs/{id}` | Yes | Get blog |
| PUT | `/v1/blogs/{id}` | Yes | Update blog |
| DELETE | `/v1/blogs/{id}` | Yes | Delete blog |
| GET | `/v1/blogs/{id}/comments` | Yes | List blog comments |
| POST | `/v1/blogs/{id}/comments` | Yes | Add blog comment |
| DELETE | `/v1/blogs/comments/{id}` | Yes | Delete blog comment |
| POST | `/v1/blogs/upload-image` | Yes | Upload image for blog editor |
| POST | `/v1/blogs/{id}/react` | Yes | React to blog |
| POST | `/v1/blogs/comments/{id}/react` | Yes | React to blog comment |

### Forums
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/forums/sections` | Yes | List forum sections |
| GET | `/v1/forums/{id}/threads` | Yes | List threads in forum |
| POST | `/v1/forums/{id}/threads` | Yes | Create thread |
| GET | `/v1/forums/threads/{id}` | Yes | Get thread |
| PUT | `/v1/forums/threads/{id}` | Yes | Update thread |
| DELETE | `/v1/forums/threads/{id}` | Yes | Delete thread |
| GET | `/v1/forums/threads/{id}/replies` | Yes | List thread replies |
| POST | `/v1/forums/threads/{id}/replies` | Yes | Create reply |
| PUT | `/v1/forums/replies/{id}` | Yes | Update reply |
| DELETE | `/v1/forums/replies/{id}` | Yes | Delete reply |
| POST | `/v1/forums/threads/{id}/vote` | Yes | Vote on thread |
| POST | `/v1/forums/threads/{id}/share` | Yes | Share thread |

### Movies
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/movies` | Yes | List movies |
| POST | `/v1/movies` | Yes | Create movie |
| GET | `/v1/movies/{id}` | Yes | Get movie |
| PUT | `/v1/movies/{id}` | Yes | Update movie |
| DELETE | `/v1/movies/{id}` | Yes | Delete movie |
| GET | `/v1/movies/{id}/comments` | Yes | List movie comments |
| POST | `/v1/movies/{id}/comments` | Yes | Add movie comment |
| POST | `/v1/movies/{id}/react` | Yes | React to movie |
| POST | `/v1/movies/{id}/watch` | Yes | Record movie watch |

### Games
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/games` | No | List games |
| GET | `/v1/games/my` | Yes | My recently played games |
| GET | `/v1/games/{id}` | No | Get game |
| POST | `/v1/games/{id}/play` | Yes | Record game play |

### Custom Pages (Public)
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/pages/custom` | No | List custom pages |
| GET | `/v1/pages/custom/{slug}` | No | Get custom page by slug |

### GIF Proxy
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/gifs/search` | Yes | Search/trending GIFs through Giphy or Tenor (server-side keys; query: `q`, `limit`, `offset`, optional `provider=giphy\|tenor`) |

### AI Assistance
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/v1/ai/chat-suggestions` | Yes | Generate smart-reply chips for a chat thread (body: `{conversation_id, recent_messages[]}`) |

---

## Commerce Service

### Products
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/products` | Yes | List products |
| POST | `/v1/products` | Yes | Create product |
| GET | `/v1/products/search` | Yes | Search products |
| GET | `/v1/products/my` | Yes | My products |
| GET | `/v1/products/categories` | No | List product categories |
| GET | `/v1/products/{id}` | Yes | Get product |
| PUT | `/v1/products/{id}` | Yes | Update product |
| DELETE | `/v1/products/{id}` | Yes | Delete product |
| GET | `/v1/products/{id}/reviews` | Yes | List product reviews |
| POST | `/v1/products/{id}/reviews` | Yes | Add product review |
| POST | `/v1/products/nearby` | Yes | Nearby products (body: `{lat, lng}`) |

### Cart
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/cart` | Yes | Get cart |
| POST | `/v1/cart` | Yes | Add item to cart |
| DELETE | `/v1/cart` | Yes | Clear cart |
| PUT | `/v1/cart/{id}` | Yes | Update cart item quantity |
| DELETE | `/v1/cart/{id}` | Yes | Remove item from cart |

### Orders
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/v1/orders` | Yes | Create order |
| GET | `/v1/orders/my` | Yes | My orders (as buyer) |
| GET | `/v1/orders/sales` | Yes | My sales (as seller) |
| GET | `/v1/orders/{id}` | Yes | Get order |
| PUT | `/v1/orders/{id}/status` | Yes | Update order status |
| GET | `/v1/orders/{id}/tracking` | Yes | Get order tracking |
| POST | `/v1/orders/{id}/refund` | Yes | Request order refund |

### Jobs
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/jobs` | Yes | List jobs |
| POST | `/v1/jobs` | Yes | Create job listing |
| GET | `/v1/jobs/my` | Yes | My job listings |
| GET | `/v1/jobs/applied` | Yes | Jobs I applied to |
| GET | `/v1/jobs/search` | Yes | Search jobs |
| GET | `/v1/jobs/categories` | No | List job categories |
| GET | `/v1/jobs/{id}` | Yes | Get job |
| PUT | `/v1/jobs/{id}` | Yes | Update job |
| DELETE | `/v1/jobs/{id}` | Yes | Delete job |
| POST | `/v1/jobs/{id}/apply` | Yes | Apply to job |
| GET | `/v1/jobs/{id}/applications` | Yes | List job applications |
| PUT | `/v1/jobs/applications/{id}/status` | Yes | Update application status |

### Funding
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/fundings` | Yes | List funding campaigns |
| POST | `/v1/fundings` | Yes | Create funding campaign |
| GET | `/v1/fundings/my` | Yes | My campaigns |
| GET | `/v1/fundings/{id}` | Yes | Get campaign |
| PUT | `/v1/fundings/{id}` | Yes | Update campaign |
| DELETE | `/v1/fundings/{id}` | Yes | Delete campaign |
| POST | `/v1/fundings/{id}/donate` | Yes | Donate to campaign |
| GET | `/v1/fundings/{id}/donations` | Yes | List donations |

### Offers
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/offers` | Yes | List offers |
| POST | `/v1/offers` | Yes | Create offer |
| GET | `/v1/offers/my` | Yes | My offers |
| GET | `/v1/offers/nearby` | Yes | Nearby offers |
| GET | `/v1/offers/{id}` | Yes | Get offer |
| PUT | `/v1/offers/{id}` | Yes | Update offer |
| DELETE | `/v1/offers/{id}` | Yes | Delete offer |

### Gifts
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/gifts` | Yes | List available gifts |
| GET | `/v1/gifts/categories` | No | List gift categories |
| POST | `/v1/gifts/send/{recipient_id}` | Yes | Send gift to user |
| GET | `/v1/gifts/received` | Yes | My received gifts |

### Stickers
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/stickers/packs` | Yes | List sticker packs |
| GET | `/v1/stickers/packs/{id}` | Yes | Get sticker pack |
| POST | `/v1/stickers/packs/{id}/purchase` | Yes | Purchase sticker pack |
| GET | `/v1/stickers/my` | Yes | My purchased sticker packs |


---

## Payment Service

### Payments
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/v1/payments/create` | Yes | Create payment session |
| POST | `/v1/payments/verify` | Yes | Verify payment |
| GET | `/v1/payments/history` | Yes | Payment history |
| POST | `/v1/payments/refund` | Yes | Request refund |

### Wallet
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/payments/wallet/balance` | Yes | Get wallet balance |
| POST | `/v1/payments/wallet/add` | Yes | Add funds to wallet |
| POST | `/v1/payments/wallet/transfer` | Yes | Transfer to another user |

### Withdrawals
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/v1/payments/withdraw` | Yes | Request withdrawal |
| GET | `/v1/payments/withdrawals` | Yes | List my withdrawals |
| PUT | `/v1/payments/withdrawals/{id}/status` | Yes | Update withdrawal status |

### Pro Subscriptions
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/payments/pro/plans` | No | List Pro plans |
| POST | `/v1/payments/pro/subscribe` | Yes | Subscribe to Pro plan |
| POST | `/v1/payments/pro/cancel` | Yes | Cancel Pro subscription |
| POST | `/v1/payments/pro/refund-request` | Yes | Request Pro refund |
| POST | `/v1/payments/bank-receipt` | Yes | Upload bank transfer receipt |

### Creator Subscriptions (Patreon Mode)
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/v1/payments/creator/tiers` | Yes | Create creator tier |
| PUT | `/v1/payments/creator/tiers/{id}` | Yes | Update creator tier |
| DELETE | `/v1/payments/creator/tiers/{id}` | Yes | Delete creator tier |
| GET | `/v1/payments/creator/{user_id}/tiers` | Yes | List creator's tiers |
| POST | `/v1/payments/creator/subscribe/{user_id}` | Yes | Subscribe to creator |
| DELETE | `/v1/payments/creator/subscribe/{user_id}` | Yes | Unsubscribe from creator |
| GET | `/v1/payments/creator/subscribers` | Yes | My subscribers |
| GET | `/v1/payments/creator/subscriptions` | Yes | My active subscriptions |

### Webhooks
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/v1/payments/webhooks/{provider}` | No | Payment webhook (provider = stripe\|paypal\|paystack\|coinbase\|flutterwave\|razorpay\|cashfree\|iyzipay\|yoomoney\|aamarpay\|fortumo\|coinpayments\|2checkout\|braintree\|payfast\|paysera\|securionpay\|ngenius\|paypro-bitcoin) |

---

## Media Service

### Media Upload
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/v1/media/upload` | Yes | Upload media file (multipart) |
| POST | `/v1/media/upload/avatar` | Yes | Upload avatar |
| POST | `/v1/media/upload/cover` | Yes | Upload cover photo |
| GET | `/v1/media/{id}` | Yes | Get media info |
| DELETE | `/v1/media/{id}` | Yes | Delete media |
| GET | `/v1/media/my` | Yes | My uploaded media |
| POST | `/v1/media/{id}/rotate` | Yes | Rotate an image asset 90 / 180 / 270 / -90 (body: `{degrees}`); generates a new derivative and updates URLs |

### Stories
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/stories` | Yes | List stories feed |
| POST | `/v1/stories` | Yes | Create story |
| GET | `/v1/stories/my` | Yes | My stories |
| GET | `/v1/stories/archive` | Yes | Archived stories |
| GET | `/v1/stories/{id}` | Yes | Get story |
| DELETE | `/v1/stories/{id}` | Yes | Delete story |
| POST | `/v1/stories/{id}/view` | Yes | Mark story as viewed |
| GET | `/v1/stories/{id}/viewers` | Yes | List story viewers |
| POST | `/v1/stories/{id}/react` | Yes | React to story |
| GET | `/v1/stories/{id}/reactions` | Yes | List story reactions |
| POST | `/v1/stories/{id}/reply` | Yes | Reply to story (sends DM) |

---

## Realtime Service

### WebSocket
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/ws` | Yes (query param `?token=`) | WebSocket connection |

### Presence
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/presence/online` | Yes | List online users |
| GET | `/v1/presence/{user_id}` | Yes | Check if user is online |

### WebSocket Protocol (client ↔ server)
The realtime service uses a JSON envelope: `{ "type": "<event>", "data": {...} }`.

Client → server control frames:

| Type | Payload | Description |
|------|---------|-------------|
| `subscribe` | `{topic}` | Subscribe the connection to a topic (e.g. `chat.<conversation_id>`, `presence.<user_id>`, `feed.global`) |
| `unsubscribe` | `{topic}` | Drop a topic subscription |
| `ping` | `{}` | Heartbeat; server replies with `pong` |

### WebSocket Events (server → client)
| Event | Payload | Description |
|-------|---------|-------------|
| `message.new` | `{conversation_id, message}` | New message received |
| `notification.new` | `{notification}` | New notification |
| `notification.unread_count` | `{count}` | Unread notification count for the current user |
| `presence.online` | `{user_id, last_seen}` | User came online |
| `presence.offline` | `{user_id, last_seen}` | User went offline |
| `typing.start` | `{conversation_id, user_id}` | User started typing |
| `typing.stop` | `{conversation_id, user_id}` | User stopped typing |
| `call.incoming` / `call.started` | `{call_id, caller_id, callee_id, call_type}` | Incoming/started call |
| `call.answered` | `{call_id}` | Call accepted by callee |
| `call.ended` | `{call_id}` | Call ended (declined, hung up, missed) |
| `post.new` | `{post_id, author_id}` | New post available in feed |
| `post.reaction` | `{post_id, user_id, reaction_type}` | Reaction added/changed on a post |
| `post.comment` | `{post_id, comment}` | New comment on a post the user follows |
| `feed.new_posts` | `{count}` | "Show new posts" banner counter (drives `NewPostsBanner` widget) |
| `user.avatar_changed` | `{user_id, avatar_url}` | Avatar updated; clients refresh attached UI without reload |
| `user.name_changed` | `{user_id, first_name, last_name}` | Display name updated |
| `social.follow` | `{follower_id, followed_id}` | New follow relationship |
| `chat.color_changed` | `{conversation_id, color}` | Conversation color updated |


---

## Admin Service

### Dashboard
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/admin/dashboard` | Admin | Dashboard stats |
| GET | `/v1/admin/dashboard/charts` | Admin | Chart data |
| GET | `/v1/admin/dashboard/top-countries` | Admin | Top countries |
| GET | `/v1/admin/system-info` | Admin | System info |

### User Management
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/admin/users` | Admin | List all users |
| GET | `/v1/admin/users/{id}` | Admin | Get user details |
| PUT | `/v1/admin/users/{id}` | Admin | Update user |
| POST | `/v1/admin/users/{id}/ban` | Admin | Ban user |
| POST | `/v1/admin/users/{id}/unban` | Admin | Unban user |
| POST | `/v1/admin/users/{id}/verify` | Admin | Verify user |
| DELETE | `/v1/admin/users/{id}` | Admin | Delete user |
| POST | `/v1/admin/users/{user_id}/make-admin` | Admin | Grant admin role |
| POST | `/v1/admin/users/{user_id}/remove-admin` | Admin | Remove admin role |
| POST | `/v1/admin/users/{user_id}/make-pro` | Admin | Grant Pro status |
| POST | `/v1/admin/users/{user_id}/remove-pro` | Admin | Remove Pro status |
| GET | `/v1/admin/users/{user_id}/permissions` | Admin | Get user permissions |
| PUT | `/v1/admin/users/{user_id}/permissions` | Admin | Update user permissions |
| POST | `/v1/admin/users/{user_id}/top-up` | Admin | Top up user wallet |
| DELETE | `/v1/admin/users/{user_id}/content` | Admin | Delete all user content |
| POST | `/v1/admin/send-email` | Admin | Send email to user |
| GET | `/v1/admin/pro-members` | Admin | List Pro members |
| GET | `/v1/admin/online-users` | Admin | List online users |
| GET | `/v1/admin/referrals` | Admin | List referrals |

### Reports & Moderation
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/admin/reports` | Admin | List reports |
| GET | `/v1/admin/reports/{id}` | Admin | Get report |
| POST | `/v1/admin/reports/{id}/resolve` | Admin | Resolve report |
| POST | `/v1/admin/reports/{id}/dismiss` | Admin | Dismiss report |
| GET | `/v1/admin/moderation/posts` | Admin | Pending posts |
| POST | `/v1/admin/moderation/posts/{id}/approve` | Admin | Approve post |
| POST | `/v1/admin/moderation/posts/{id}/reject` | Admin | Reject post |
| DELETE | `/v1/admin/posts/{id}` | Admin | Hard delete post |
| GET | `/v1/admin/moderation/blogs` | Admin | Pending blogs |
| POST | `/v1/admin/moderation/blogs/{id}/approve` | Admin | Approve blog |
| POST | `/v1/admin/moderation/blogs/{id}/reject` | Admin | Reject blog |
| GET | `/v1/admin/verifications` | Admin | Verification requests |
| POST | `/v1/admin/verifications/{id}/approve` | Admin | Approve verification |
| POST | `/v1/admin/verifications/{id}/reject` | Admin | Reject verification |
| GET | `/v1/admin/banned-ips` | Admin | List banned IPs |
| POST | `/v1/admin/banned-ips` | Admin | Ban IP |
| DELETE | `/v1/admin/banned-ips/{id}` | Admin | Unban IP |

### Configuration
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/admin/config` | Admin | List all config |
| GET | `/v1/admin/config/{category}` | Admin | Get config category |
| PUT | `/v1/admin/config` | Admin | Update config |
| GET | `/v1/admin/settings/{category}` | Admin | Get settings by category |
| PUT | `/v1/admin/settings/{category}` | Admin | Update settings by category |

### Payments Admin
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/admin/payments/stats` | Admin | Payment statistics |
| GET | `/v1/admin/payments/transactions` | Admin | List transactions |
| GET | `/v1/admin/payments/withdrawals` | Admin | Pending withdrawals |
| POST | `/v1/admin/payments/withdrawals/{id}/approve` | Admin | Approve withdrawal |
| POST | `/v1/admin/payments/withdrawals/{id}/reject` | Admin | Reject withdrawal |
| GET | `/v1/admin/payments/pro-plans` | Admin | List Pro plans |
| POST | `/v1/admin/payments/pro-plans` | Admin | Create/update Pro plan |
| GET | `/v1/admin/refunds` | Admin | List refund requests |
| POST | `/v1/admin/refunds/{id}/approve` | Admin | Approve refund |
| POST | `/v1/admin/refunds/{id}/reject` | Admin | Reject refund |
| GET | `/v1/admin/bank-receipts` | Admin | List bank receipts |
| POST | `/v1/admin/bank-receipts/{id}/approve` | Admin | Approve bank receipt |
| POST | `/v1/admin/bank-receipts/{id}/reject` | Admin | Reject bank receipt |

### Content Management
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/admin/site-pages` | Admin | List pages |
| DELETE | `/v1/admin/site-pages/{id}` | Admin | Delete page |
| GET | `/v1/admin/site-groups` | Admin | List groups |
| DELETE | `/v1/admin/site-groups/{id}` | Admin | Delete group |
| GET | `/v1/admin/site-blogs` | Admin | List blogs |
| POST | `/v1/admin/site-blogs/{id}/approve` | Admin | Approve blog |
| DELETE | `/v1/admin/site-blogs/{id}` | Admin | Delete blog |
| GET | `/v1/admin/site-products` | Admin | List products |
| DELETE | `/v1/admin/site-products/{id}` | Admin | Delete product |
| GET | `/v1/admin/site-jobs` | Admin | List jobs |
| DELETE | `/v1/admin/site-jobs/{id}` | Admin | Delete job |
| GET | `/v1/admin/site-funding` | Admin | List funding campaigns |
| DELETE | `/v1/admin/site-funding/{id}` | Admin | Delete campaign |
| GET | `/v1/admin/site-events` | Admin | List events |
| DELETE | `/v1/admin/site-events/{id}` | Admin | Delete event |
| GET | `/v1/admin/site-forums` | Admin | List forums |
| PUT | `/v1/admin/site-forums/{id}` | Admin | Update forum |
| DELETE | `/v1/admin/site-forums/{id}` | Admin | Delete forum |
| GET | `/v1/admin/manage-posts` | Admin | List all posts |
| GET | `/v1/admin/stories` | Admin | List stories |
| POST | `/v1/admin/stories/{id}/hide` | Admin | Hide story |
| DELETE | `/v1/admin/stories/{id}` | Admin | Delete story |
| GET | `/v1/admin/offers` | Admin | List offers |
| DELETE | `/v1/admin/offers/{id}` | Admin | Delete offer |
| GET | `/v1/admin/orders` | Admin | List orders |
| GET | `/v1/admin/reviews` | Admin | List reviews |
| DELETE | `/v1/admin/reviews/{id}` | Admin | Delete review |

### Localization
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/admin/languages` | Admin | List languages |
| POST | `/v1/admin/languages` | Admin | Create language |
| PUT | `/v1/admin/languages/{id}` | Admin | Update language |
| DELETE | `/v1/admin/languages/{id}` | Admin | Delete language |
| GET | `/v1/admin/translations` | Admin | List translations |
| POST | `/v1/admin/translations` | Admin | Upsert translation |
| POST | `/v1/admin/translations/bulk` | Admin | Bulk upsert translations |
| DELETE | `/v1/admin/translations/{id}` | Admin | Delete translation |

### Customization
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/admin/categories` | Admin | List categories |
| POST | `/v1/admin/categories` | Admin | Create category |
| PUT | `/v1/admin/categories/{id}` | Admin | Update category |
| DELETE | `/v1/admin/categories/{id}` | Admin | Delete category |
| GET | `/v1/admin/sub-categories` | Admin | List sub-categories |
| POST | `/v1/admin/sub-categories` | Admin | Create sub-category |
| PUT | `/v1/admin/sub-categories/{id}` | Admin | Update sub-category |
| DELETE | `/v1/admin/sub-categories/{id}` | Admin | Delete sub-category |
| GET | `/v1/admin/colored-posts` | Admin | List colored post templates |
| POST | `/v1/admin/colored-posts` | Admin | Create colored post template |
| PUT | `/v1/admin/colored-posts/{id}` | Admin | Update colored post template |
| DELETE | `/v1/admin/colored-posts/{id}` | Admin | Delete colored post template |
| GET | `/v1/admin/reaction-types` | Admin | List reaction types |
| POST | `/v1/admin/reaction-types` | Admin | Create reaction type |
| PUT | `/v1/admin/reaction-types/{id}` | Admin | Update reaction type |
| DELETE | `/v1/admin/reaction-types/{id}` | Admin | Delete reaction type |
| GET | `/v1/admin/gifts` | Admin | List gifts |
| POST | `/v1/admin/gifts` | Admin | Create gift |
| PUT | `/v1/admin/gifts/{id}` | Admin | Update gift |
| DELETE | `/v1/admin/gifts/{id}` | Admin | Delete gift |
| GET | `/v1/admin/sticker-packs` | Admin | List sticker packs |
| POST | `/v1/admin/sticker-packs` | Admin | Create sticker pack |
| PUT | `/v1/admin/sticker-packs/{id}` | Admin | Update sticker pack |
| DELETE | `/v1/admin/sticker-packs/{id}` | Admin | Delete sticker pack |
| GET | `/v1/admin/sticker-packs/{pack_id}/stickers` | Admin | List stickers in pack |
| POST | `/v1/admin/sticker-packs/{pack_id}/stickers` | Admin | Add sticker to pack |
| DELETE | `/v1/admin/stickers/{id}` | Admin | Delete sticker |
| GET | `/v1/admin/email-templates` | Admin | List email templates |
| POST | `/v1/admin/email-templates` | Admin | Create email template |
| PUT | `/v1/admin/email-templates/{id}` | Admin | Update email template |
| DELETE | `/v1/admin/email-templates/{id}` | Admin | Delete email template |
| GET | `/v1/admin/profile-fields` | Admin | List profile fields |
| POST | `/v1/admin/profile-fields` | Admin | Create profile field |
| PUT | `/v1/admin/profile-fields/{id}` | Admin | Update profile field |
| DELETE | `/v1/admin/profile-fields/{id}` | Admin | Delete profile field |
| GET | `/v1/admin/pages` | Admin | List custom pages |
| POST | `/v1/admin/pages` | Admin | Create custom page |
| PUT | `/v1/admin/pages/{id}` | Admin | Update custom page |
| DELETE | `/v1/admin/pages/{id}` | Admin | Delete custom page |
| GET | `/v1/admin/pages/slug/{slug}` | Admin | Get custom page by slug |
| GET | `/v1/admin/terms-pages` | Admin | List terms pages |
| PUT | `/v1/admin/terms-pages/{id}` | Admin | Update terms page |
| GET | `/v1/admin/genders` | Admin | List genders |
| POST | `/v1/admin/genders` | Admin | Create gender |
| PUT | `/v1/admin/genders/{id}` | Admin | Update gender |
| DELETE | `/v1/admin/genders/{id}` | Admin | Delete gender |
| GET | `/v1/admin/currencies` | Admin | List currencies |
| POST | `/v1/admin/currencies` | Admin | Create currency |
| PUT | `/v1/admin/currencies/{id}` | Admin | Update currency |
| POST | `/v1/admin/currencies/{id}/toggle` | Admin | Toggle currency active |
| DELETE | `/v1/admin/currencies/{id}` | Admin | Delete currency |

### System
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/admin/backups` | Admin | List backups |
| POST | `/v1/admin/backups/trigger` | Admin | Trigger backup |
| GET | `/v1/admin/backups/{id}/download` | Admin | Download a backup file |
| POST | `/v1/admin/backups/{id}/restore` | Admin | Restore a previously taken backup |
| DELETE | `/v1/admin/backups/{id}` | Admin | Delete a backup file |
| GET | `/v1/admin/system/ffmpeg-probe` | Admin | Probe ffmpeg/ffprobe availability + version |
| POST | `/v1/admin/system/email/test` | Admin | Send a test email through the configured provider |
| POST | `/v1/admin/system/sms/test` | Admin | Send a test SMS through the configured provider |
| POST | `/v1/admin/oauth-apps/{provider}/verify` | Admin | Verify OAuth provider credentials with a `me`/`userinfo` round-trip |
| GET | `/v1/admin/system/dlq` | Admin | List failed/skipped domain events (DLQ) |
| POST | `/v1/admin/system/dlq/{id}/retry` | Admin | Republish a failed domain event |
| DELETE | `/v1/admin/system/dlq/{id}` | Admin | Discard a failed event without retrying |
| POST | `/v1/admin/storage/config/{id}/test` | Admin | Probe storage credentials (PutObject/DeleteObject) |
| GET | `/v1/admin/newsletter/subscribers` | Admin | List newsletter subscribers |
| DELETE | `/v1/admin/newsletter/subscribers/{id}` | Admin | Remove subscriber |
| POST | `/v1/admin/newsletter/send` | Admin | Send newsletter |
| GET | `/v1/admin/announcements` | Admin | List announcements |
| POST | `/v1/admin/announcements` | Admin | Create announcement |
| PUT | `/v1/admin/announcements/{id}` | Admin | Update announcement |
| DELETE | `/v1/admin/announcements/{id}` | Admin | Delete announcement |
| GET | `/v1/admin/invitations` | Admin | List invitations |
| POST | `/v1/admin/invitations` | Admin | Create invitation |
| DELETE | `/v1/admin/invitations/{id}` | Admin | Delete invitation |
| GET | `/v1/admin/oauth-apps` | Admin | List OAuth apps |
| POST | `/v1/admin/oauth-apps/{id}/toggle` | Admin | Toggle OAuth app |
| DELETE | `/v1/admin/oauth-apps/{id}` | Admin | Delete OAuth app |
| GET | `/v1/admin/activities` | Admin | Activity log |
| GET | `/v1/admin/ads` | Admin | List user ads |
| PUT | `/v1/admin/ads/{id}` | Admin | Update user ad |
| GET | `/v1/admin/user-ads` | Admin | List all user ads |
| POST | `/v1/admin/user-ads/{id}/toggle` | Admin | Toggle user ad |
| DELETE | `/v1/admin/user-ads/{id}` | Admin | Delete user ad |
| GET | `/v1/admin/mass-notifications` | Admin | List mass notifications |
| POST | `/v1/admin/mass-notifications/send` | Admin | Send mass notification |
| POST | `/v1/admin/sitemap/generate` | Admin | Generate sitemap |
| GET | `/v1/admin/fake-users` | Admin | List fake users |
| POST | `/v1/admin/fake-users` | Admin | Create fake user |
| GET | `/v1/admin/api-keys` | Admin | List API keys |
| POST | `/v1/admin/api-keys` | Admin | Create API key |
| POST | `/v1/admin/api-keys/{id}/toggle` | Admin | Toggle API key |
| DELETE | `/v1/admin/api-keys/{id}` | Admin | Delete API key |
| GET | `/v1/admin/monetization` | Admin | List monetization subscriptions |

### Forum Admin
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/admin/forum-sections` | Admin | List forum sections |
| POST | `/v1/admin/forum-sections` | Admin | Create forum section |
| PUT | `/v1/admin/forum-sections/{id}` | Admin | Update forum section |
| DELETE | `/v1/admin/forum-sections/{id}` | Admin | Delete forum section |
| POST | `/v1/admin/forums` | Admin | Create forum |
| GET | `/v1/admin/forum-threads` | Admin | List forum threads |
| DELETE | `/v1/admin/forum-threads/{id}` | Admin | Delete forum thread |
| GET | `/v1/admin/forum-replies` | Admin | List forum replies |
| DELETE | `/v1/admin/forum-replies/{id}` | Admin | Delete forum reply |

### Movies & Games Admin
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/admin/movies` | Admin | List movies |
| POST | `/v1/admin/movies` | Admin | Create movie |
| PUT | `/v1/admin/movies/{id}` | Admin | Update movie |
| POST | `/v1/admin/movies/{id}/approve` | Admin | Approve movie |
| POST | `/v1/admin/movies/{id}/feature` | Admin | Feature movie |
| DELETE | `/v1/admin/movies/{id}` | Admin | Delete movie |
| GET | `/v1/admin/games` | Admin | List games |
| POST | `/v1/admin/games` | Admin | Create game |
| POST | `/v1/admin/games/{id}/toggle` | Admin | Toggle game active |
| DELETE | `/v1/admin/games/{id}` | Admin | Delete game |

### Advanced Settings
| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/v1/admin/auto-settings` | Admin | Get auto settings |
| PUT | `/v1/admin/auto-settings/auto-delete` | Admin | Update auto-delete settings |
| POST | `/v1/admin/auto-settings/friends` | Admin | Add auto-friend |
| DELETE | `/v1/admin/auto-settings/friends/{id}` | Admin | Remove auto-friend |
| POST | `/v1/admin/auto-settings/joins` | Admin | Add auto-join |
| DELETE | `/v1/admin/auto-settings/joins/{id}` | Admin | Remove auto-join |
| POST | `/v1/admin/auto-settings/likes` | Admin | Add auto-like |
| DELETE | `/v1/admin/auto-settings/likes/{id}` | Admin | Remove auto-like |
| GET | `/v1/admin/custom-code` | Admin | Get custom code (header/footer) |
| PUT | `/v1/admin/custom-code` | Admin | Update custom code |
| GET | `/v1/admin/site-ads` | Admin | List site ads |
| POST | `/v1/admin/site-ads` | Admin | Create site ad |
| PUT | `/v1/admin/site-ads/{id}` | Admin | Update site ad |
| DELETE | `/v1/admin/site-ads/{id}` | Admin | Delete site ad |

---

## Common Query Parameters

| Parameter | Type | Description |
|-----------|------|-------------|
| `cursor` | string | Cursor for pagination |
| `limit` | int | Items per page (default: 20, max: 50) |
| `q` | string | Search query |
| `type` | string | Filter by type |
| `sort` | string | Sort field |
| `order` | string | `asc` or `desc` |

## Common Response Format

```json
{
  "data": { ... },
  "meta": {
    "cursor": "next_cursor_string",
    "has_more": true,
    "total": 100
  }
}
```

## Error Response Format

```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Human readable message",
    "details": {
      "field_name": ["error message"]
    }
  }
}
```

## HTTP Status Codes

| Code | Meaning |
|------|---------|
| 200 | Success |
| 201 | Created |
| 400 | Bad Request |
| 401 | Unauthorized (token missing/expired) |
| 403 | Forbidden (insufficient permissions) |
| 404 | Not Found |
| 409 | Conflict (duplicate) |
| 422 | Validation Error |
| 429 | Rate Limited |
| 500 | Internal Server Error |
