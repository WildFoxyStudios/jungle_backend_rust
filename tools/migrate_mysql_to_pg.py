#!/usr/bin/env python3
"""
MySQL → PostgreSQL Data Migration Script for WoWonder
=====================================================
Reads from the original WoWonder MySQL database and inserts into
the new PostgreSQL schema. Handles type conversions, table consolidations,
and FK ordering.

Requirements:
  pip install mysql-connector-python psycopg2-binary

Usage:
  python migrate_mysql_to_pg.py \
    --mysql-host 127.0.0.1 --mysql-port 3306 --mysql-db wowonder_db \
    --mysql-user root --mysql-pass "" \
    --pg-host 127.0.0.1 --pg-port 5432 --pg-db wowonder \
    --pg-user postgres --pg-pass postgres \
    --batch-size 1000
"""

import argparse
import sys
import json
import datetime
from decimal import Decimal

import mysql.connector
import psycopg2
import psycopg2.extras


def ts(val):
    """Convert unix timestamp or None to datetime."""
    if val is None or val == 0:
        return None
    try:
        return datetime.datetime.utcfromtimestamp(int(val))
    except (ValueError, OSError):
        return None


def boolish(val):
    """Convert '0'/'1'/int to bool."""
    if val is None:
        return False
    return str(val) in ('1', 'true', 'True', 'yes')


def safe_json(val):
    """Parse JSON string or return empty dict."""
    if val is None or val == '':
        return {}
    if isinstance(val, dict):
        return val
    try:
        return json.loads(val)
    except (json.JSONDecodeError, TypeError):
        return {}


def safe_str(val, default=''):
    if val is None:
        return default
    return str(val)


class Migrator:
    def __init__(self, mysql_cfg, pg_cfg, batch_size=1000):
        self.batch = batch_size
        self.my = mysql.connector.connect(**mysql_cfg, charset='utf8mb4')
        self.my.autocommit = False
        self.pg = psycopg2.connect(**pg_cfg)
        self.pg.autocommit = False
        self.stats = {}

    def log(self, msg):
        print(f"[MIGRATE] {msg}", flush=True)

    def count(self, table):
        cur = self.my.cursor()
        cur.execute(f"SELECT COUNT(*) FROM {table}")
        c = cur.fetchone()[0]
        cur.close()
        return c

    def fetch_all(self, query):
        cur = self.my.cursor(dictionary=True)
        cur.execute(query)
        rows = cur.fetchall()
        cur.close()
        return rows

    def run(self):
        self.log("Starting MySQL → PostgreSQL migration")
        steps = [
            self.migrate_users,
            self.migrate_sessions,
            self.migrate_follows,
            self.migrate_blocks,
            self.migrate_categories,
            self.migrate_pages,
            self.migrate_page_likes,
            self.migrate_groups,
            self.migrate_group_members,
            self.migrate_events,
            self.migrate_posts,
            self.migrate_comments,
            self.migrate_reactions,
            self.migrate_conversations,
            self.migrate_messages,
            self.migrate_blogs,
            self.migrate_blog_comments,
            self.migrate_forums,
            self.migrate_products,
            self.migrate_jobs,
            self.migrate_funding,
            self.migrate_notifications,
            self.migrate_stories,
            self.migrate_config,
            self.migrate_translations,
            self.migrate_movies,
            self.migrate_games,
            self.migrate_hashtags,
            self.migrate_event_responses,
            self.migrate_page_admins,
            self.migrate_page_ratings,
            self.migrate_mutes,
            self.migrate_pokes,
            self.migrate_saved_posts,
            self.migrate_hidden_posts,
            self.migrate_story_media,
            self.migrate_story_views,
            self.migrate_offers,
            self.migrate_payments,
            self.migrate_ads,
            self.migrate_reports,
            self.migrate_announcements,
            self.migrate_verification_requests,
            self.migrate_activities,
            self.migrate_calls,
            self.migrate_stickers,
            self.migrate_gifts,
            self.migrate_banned_ips,
            self.migrate_login_attempts,
            self.migrate_user_experience,
            self.migrate_user_certifications,
            self.migrate_user_skills,
            self.migrate_recent_searches,
            self.migrate_colored_post_templates,
            self.migrate_reaction_types,
            self.migrate_invitation_links,
            self.migrate_custom_pages,
            self.migrate_email_templates,
            self.migrate_polls,
            self.migrate_orders,
            self.migrate_job_applications,
            self.migrate_funding_donations,
            self.migrate_product_reviews,
            self.migrate_profile_fields,
            self.migrate_uploaded_media,
        ]
        for step in steps:
            try:
                step()
                self.pg.commit()
            except Exception as e:
                self.pg.rollback()
                self.log(f"ERROR in {step.__name__}: {e}")
                raise

        self.reset_sequences()
        self.verify_migration()
        self.log("Migration complete!")
        for table, count in sorted(self.stats.items()):
            self.log(f"  {table}: {count} rows")

    def _insert_batch(self, table, columns, rows):
        if not rows:
            return
        cur = self.pg.cursor()
        cols = ', '.join(columns)
        placeholders = ', '.join(['%s'] * len(columns))
        query = f"INSERT INTO {table} ({cols}) VALUES ({placeholders}) ON CONFLICT DO NOTHING"
        psycopg2.extras.execute_batch(cur, query, rows, page_size=self.batch)
        cur.close()
        self.stats[table] = self.stats.get(table, 0) + len(rows)

    # ── Users ──
    def migrate_users(self):
        total = self.count('Wo_Users')
        self.log(f"Migrating {total} users...")
        rows = self.fetch_all("SELECT * FROM Wo_Users ORDER BY user_id ASC")
        batch = []
        for r in rows:
            privacy = {}
            for key in ['follow_privacy', 'message_privacy', 'post_privacy', 'birth_privacy']:
                if key in r:
                    privacy[key] = safe_str(r.get(key, ''))

            social = {}
            for provider in ['google', 'facebook', 'twitter', 'linkedin', 'vkontakte', 'instagram', 'discord']:
                pid = r.get(f'{provider}', '')
                if pid:
                    social[provider] = {'id': str(pid)}

            batch.append((
                r['user_id'], r.get('username', ''), r.get('email', ''),
                safe_str(r.get('password', '')),
                safe_str(r.get('first_name', '')), safe_str(r.get('last_name', '')),
                safe_str(r.get('avatar', '')), safe_str(r.get('cover', '')),
                safe_str(r.get('about', '')), safe_str(r.get('gender', '')),
                safe_str(r.get('birthday', '')),
                safe_str(r.get('phone_number', '')),
                safe_str(r.get('country_id', '')),
                safe_str(r.get('city', '')),
                safe_str(r.get('working', '')),
                safe_str(r.get('school', '')),
                safe_str(r.get('website', '')),
                boolish(r.get('verified', 0)),
                boolish(r.get('admin', 0)),
                boolish(r.get('is_pro', 0)),
                safe_str(r.get('pro_type', '')),
                ts(r.get('pro_time', 0)),
                Decimal(str(r.get('wallet', 0) or 0)),
                Decimal(str(r.get('balance', 0) or 0)),
                int(r.get('points', 0) or 0),
                json.dumps(privacy),
                json.dumps(social),
                json.dumps(safe_json(r.get('notification_settings', '{}'))),
                boolish(r.get('active', 1)),
                boolish(r.get('two_factor', 0)),
                ts(r.get('lastseen', 0)),
                ts(r.get('registered', 0)),
            ))
        self._insert_batch('users', [
            'id', 'username', 'email', 'password',
            'first_name', 'last_name', 'avatar', 'cover',
            'about', 'gender', 'birthday', 'phone_number',
            'country', 'city', 'working', 'school', 'website',
            'is_verified', 'is_admin', 'is_pro', 'pro_type', 'pro_expires_at',
            'wallet', 'balance', 'points',
            'privacy_settings', 'social_logins', 'notification_settings',
            'is_active', 'two_factor_enabled', 'last_active', 'created_at',
        ], batch)
        self.log(f"  Users: {len(batch)} migrated")

    # ── Sessions ──
    def migrate_sessions(self):
        total = self.count('Wo_AppsSessions')
        self.log(f"Migrating {total} sessions...")
        rows = self.fetch_all("SELECT * FROM Wo_AppsSessions ORDER BY id ASC")
        batch = []
        for r in rows:
            batch.append((
                r['id'], r.get('user_id'),
                safe_str(r.get('session_id', '')),
                safe_str(r.get('platform', 'web')),
                ts(r.get('time', 0)),
            ))
        self._insert_batch('sessions', ['id', 'user_id', 'token_hash', 'platform', 'created_at'], batch)

    # ── Follows ──
    def migrate_follows(self):
        self.log("Migrating follows...")
        rows = self.fetch_all("SELECT * FROM Wo_Followers ORDER BY id ASC")
        batch = [(r['id'], r['follower_id'], r['following_id'], 'active',
                  ts(r.get('time', 0))) for r in rows]
        self._insert_batch('follows', ['id', 'follower_id', 'following_id', 'status', 'created_at'], batch)

    # ── Blocks ──
    def migrate_blocks(self):
        self.log("Migrating blocks...")
        rows = self.fetch_all("SELECT * FROM Wo_Blocks ORDER BY id ASC")
        batch = [(r['id'], r['blocker_id'], r['blocked_id'], ts(r.get('time', 0))) for r in rows]
        self._insert_batch('blocks', ['id', 'blocker_id', 'blocked_id', 'created_at'], batch)

    # ── Categories ──
    def migrate_categories(self):
        self.log("Migrating categories...")
        cat_tables = [
            ('Wo_Categories', 'page'), ('Wo_BlogCategories', 'blog'),
            ('Wo_Group_Category', 'group'), ('Wo_Products_Categories', 'product'),
            ('Wo_Movies_Category', 'movie'), ('Wo_Job_Category', 'job'),
        ]
        batch = []
        cid = 1
        for table, cat_type in cat_tables:
            try:
                rows = self.fetch_all(f"SELECT * FROM {table} ORDER BY id ASC")
                for r in rows:
                    name = r.get('name') or r.get('lang_key') or r.get('category_name', '')
                    batch.append((cid, cat_type, safe_str(name), '', True))
                    cid += 1
            except Exception:
                continue
        self._insert_batch('categories', ['id', 'type', 'name', 'description', 'is_active'], batch)

    # ── Pages ──
    def migrate_pages(self):
        self.log("Migrating pages...")
        rows = self.fetch_all("SELECT * FROM Wo_Pages ORDER BY page_id ASC")
        batch = []
        for r in rows:
            batch.append((
                r['page_id'], r.get('user_id'), safe_str(r.get('page_name', '')),
                safe_str(r.get('page_title', '')), safe_str(r.get('page_description', '')),
                safe_str(r.get('avatar', '')), safe_str(r.get('cover', '')),
                r.get('page_category'), int(r.get('likes', 0) or 0),
                boolish(r.get('verified', 0)), boolish(r.get('active', 1)),
                ts(r.get('registered', 0)),
            ))
        self._insert_batch('pages', [
            'page_id', 'user_id', 'page_name', 'page_title', 'page_description',
            'avatar', 'cover', 'category_id', 'like_count',
            'is_verified', 'active', 'created_at',
        ], batch)

    def migrate_page_likes(self):
        self.log("Migrating page likes...")
        rows = self.fetch_all("SELECT * FROM Wo_Pages_Likes ORDER BY id ASC")
        batch = [(r['id'], r['page_id'], r['user_id'], ts(r.get('time', 0))) for r in rows]
        self._insert_batch('page_likes', ['id', 'page_id', 'user_id', 'created_at'], batch)

    # ── Groups ──
    def migrate_groups(self):
        self.log("Migrating groups...")
        rows = self.fetch_all("SELECT * FROM Wo_Groups ORDER BY id ASC")
        batch = []
        for r in rows:
            batch.append((
                r['id'], r.get('user_id'), safe_str(r.get('group_name', '')),
                safe_str(r.get('group_title', '')), safe_str(r.get('about', '')),
                safe_str(r.get('avatar', '')), safe_str(r.get('cover', '')),
                safe_str(r.get('privacy', 'public')), r.get('category_id'),
                int(r.get('members', 0) or 0), boolish(r.get('active', 1)),
                ts(r.get('registered', 0)),
            ))
        self._insert_batch('groups', [
            'id', 'user_id', 'group_name', 'group_title', 'about',
            'avatar', 'cover', 'privacy', 'category_id',
            'member_count', 'active', 'created_at',
        ], batch)

    def migrate_group_members(self):
        self.log("Migrating group members...")
        rows = self.fetch_all("SELECT * FROM Wo_Group_Members ORDER BY id ASC")
        admin_rows = self.fetch_all("SELECT * FROM Wo_GroupAdmins")
        admin_set = {(r['group_id'], r['user_id']) for r in admin_rows}
        batch = []
        for r in rows:
            role = 'admin' if (r['group_id'], r['user_id']) in admin_set else 'member'
            batch.append((r['id'], r['group_id'], r['user_id'], role, ts(r.get('time', 0))))
        self._insert_batch('group_members', ['id', 'group_id', 'user_id', 'role', 'created_at'], batch)

    # ── Events ──
    def migrate_events(self):
        self.log("Migrating events...")
        rows = self.fetch_all("SELECT * FROM Wo_Events ORDER BY id ASC")
        batch = []
        for r in rows:
            batch.append((
                r['id'], r.get('user_id'), safe_str(r.get('name', '')),
                safe_str(r.get('description', '')), safe_str(r.get('location', '')),
                safe_str(r.get('cover', '')),
                safe_str(r.get('start_date', '')), safe_str(r.get('end_date', '')),
                ts(r.get('time', 0)),
            ))
        self._insert_batch('events', [
            'id', 'user_id', 'name', 'description', 'location', 'cover',
            'start_date', 'end_date', 'created_at',
        ], batch)

    # ── Posts ──
    def migrate_posts(self):
        total = self.count('Wo_Posts')
        self.log(f"Migrating {total} posts...")
        rows = self.fetch_all("SELECT * FROM Wo_Posts ORDER BY post_id ASC")
        batch = []
        for r in rows:
            media = []
            for key in ['postFile', 'postFileThumb', 'postFileFull']:
                val = r.get(key, '')
                if val:
                    media.append({'url': val, 'type': key})

            batch.append((
                r['post_id'], r.get('user_id'), safe_str(r.get('postText', '')),
                safe_str(r.get('postType', '')), json.dumps(media),
                safe_str(r.get('postPrivacy', 'everyone')),
                r.get('page_id'), r.get('group_id'), r.get('event_id'),
                int(r.get('postLikes', 0) or 0), int(r.get('postComments', 0) or 0),
                int(r.get('postShares', 0) or 0),
                boolish(r.get('active', 1)),
                ts(r.get('time', 0)),
            ))
        self._insert_batch('posts', [
            'id', 'user_id', 'content', 'post_type', 'media',
            'privacy', 'page_id', 'group_id', 'event_id',
            'like_count', 'comment_count', 'share_count',
            'is_approved', 'created_at',
        ], batch)

    # ── Comments (consolidated) ──
    def migrate_comments(self):
        self.log("Migrating comments...")
        batch = []
        # Main comments
        rows = self.fetch_all("SELECT * FROM Wo_Comments ORDER BY id ASC")
        for r in rows:
            batch.append((r['id'], r.get('user_id'), 'post', r.get('post_id'),
                         None, safe_str(r.get('text', '')), ts(r.get('time', 0))))
        # Replies
        max_id = max((r[0] for r in batch), default=0)
        rows = self.fetch_all("SELECT * FROM Wo_Comment_Replies ORDER BY id ASC")
        for r in rows:
            max_id += 1
            batch.append((max_id, r.get('user_id'), 'post', r.get('post_id'),
                         r.get('comment_id'), safe_str(r.get('text', '')), ts(r.get('time', 0))))
        self._insert_batch('comments', ['id', 'user_id', 'target_type', 'target_id',
                                         'parent_id', 'text', 'created_at'], batch)

    # ── Reactions (consolidated) ──
    def migrate_reactions(self):
        self.log("Migrating reactions...")
        batch = []
        rid = 0
        # Post likes
        for table, target_type, type_col in [
            ('Wo_Likes', 'post', None),
            ('Wo_Reactions', 'post', 'reaction'),
        ]:
            try:
                rows = self.fetch_all(f"SELECT * FROM {table}")
                for r in rows:
                    rid += 1
                    rtype = safe_str(r.get(type_col, 'like')) if type_col else 'like'
                    batch.append((rid, r.get('user_id'), target_type,
                                 r.get('post_id'), rtype, ts(r.get('time', 0))))
            except Exception:
                continue
        self._insert_batch('reactions', ['id', 'user_id', 'target_type', 'target_id',
                                          'reaction_type', 'created_at'], batch)

    # ── Conversations & Messages ──
    def migrate_conversations(self):
        self.log("Migrating conversations...")
        rows = self.fetch_all("SELECT * FROM Wo_UsersChat ORDER BY id ASC")
        batch_conv = []
        batch_members = []
        mid = 0
        for r in rows:
            batch_conv.append((r['id'], 'direct', ts(r.get('time', 0))))
            mid += 1
            batch_members.append((mid, r['id'], r.get('from_id'), 'member'))
            mid += 1
            batch_members.append((mid, r['id'], r.get('to_id'), 'member'))
        self._insert_batch('conversations', ['id', 'type', 'created_at'], batch_conv)
        self._insert_batch('conversation_members', ['id', 'conversation_id', 'user_id', 'role'], batch_members)

    def migrate_messages(self):
        self.log("Migrating messages...")
        rows = self.fetch_all("SELECT * FROM Wo_Messages ORDER BY id ASC")
        batch = []
        for r in rows:
            batch.append((
                r['id'], r.get('from_id'), r.get('conversation_id') or r.get('chat_id'),
                safe_str(r.get('text', '')),
                safe_str(r.get('media', '')),
                safe_str(r.get('media_type', 'text')),
                boolish(r.get('seen', 0)),
                ts(r.get('time', 0)),
            ))
        self._insert_batch('messages', ['id', 'sender_id', 'conversation_id',
                                         'text', 'media_url', 'message_type',
                                         'is_read', 'created_at'], batch)

    # ── Blogs ──
    def migrate_blogs(self):
        self.log("Migrating blogs...")
        rows = self.fetch_all("SELECT * FROM Wo_Blog ORDER BY id ASC")
        batch = []
        for r in rows:
            batch.append((
                r['id'], r.get('user'), safe_str(r.get('title', '')),
                safe_str(r.get('content', '')), safe_str(r.get('description', '')),
                safe_str(r.get('thumbnail', '')), r.get('category'),
                boolish(r.get('active', 1)),
                int(r.get('view', 0) or 0), ts(r.get('posted', 0)),
            ))
        self._insert_batch('blogs', ['id', 'user_id', 'title', 'content', 'description',
                                      'thumbnail', 'category_id', 'is_approved',
                                      'view_count', 'created_at'], batch)

    def migrate_blog_comments(self):
        self.log("Migrating blog comments...")
        batch = []
        rows = self.fetch_all("SELECT * FROM Wo_BlogComments ORDER BY id ASC")
        for r in rows:
            batch.append((r['id'], r.get('user_id'), r.get('blog_id'),
                         None, safe_str(r.get('text', '')), ts(r.get('time', 0))))
        max_id = max((r[0] for r in batch), default=0)
        try:
            rows = self.fetch_all("SELECT * FROM Wo_BlogCommentReplies ORDER BY id ASC")
            for r in rows:
                max_id += 1
                batch.append((max_id, r.get('user_id'), r.get('blog_id'),
                             r.get('comment_id'), safe_str(r.get('text', '')), ts(r.get('time', 0))))
        except Exception:
            pass
        self._insert_batch('blog_comments', ['id', 'user_id', 'blog_id',
                                              'parent_id', 'text', 'created_at'], batch)

    # ── Forums ──
    def migrate_forums(self):
        self.log("Migrating forums...")
        try:
            rows = self.fetch_all("SELECT * FROM Wo_Forum ORDER BY id ASC")
            batch = [(r['id'], safe_str(r.get('name', '')), safe_str(r.get('description', '')),
                     True) for r in rows]
            self._insert_batch('forum_sections', ['id', 'name', 'description', 'is_active'], batch)
        except Exception:
            pass

    # ── Products ──
    def migrate_products(self):
        self.log("Migrating products...")
        rows = self.fetch_all("SELECT * FROM Wo_Products ORDER BY id ASC")
        batch = []
        for r in rows:
            batch.append((
                r['id'], r.get('user_id'), safe_str(r.get('name', '')),
                safe_str(r.get('description', '')),
                Decimal(str(r.get('price', 0) or 0)),
                safe_str(r.get('currency', 'USD')),
                r.get('category'), safe_str(r.get('location', '')),
                safe_str(r.get('type', 'new')),
                'active', ts(r.get('time', 0)),
            ))
        self._insert_batch('products', ['id', 'user_id', 'name', 'description',
                                         'price', 'currency', 'category_id', 'location',
                                         'condition', 'status', 'created_at'], batch)

    def migrate_jobs(self):
        self.log("Migrating jobs...")
        rows = self.fetch_all("SELECT * FROM Wo_Job ORDER BY id ASC")
        batch = []
        for r in rows:
            batch.append((
                r['id'], r.get('user_id'), r.get('page_id'), safe_str(r.get('title', '')),
                safe_str(r.get('description', '')), safe_str(r.get('location', '')),
                safe_str(r.get('salary', '')), safe_str(r.get('job_type', '')),
                r.get('category'), boolish(r.get('active', 1)),
                ts(r.get('time', 0)),
            ))
        self._insert_batch('jobs', ['id', 'user_id', 'page_id', 'title', 'description',
                                     'location', 'salary', 'job_type', 'category_id',
                                     'is_active', 'created_at'], batch)

    def migrate_funding(self):
        self.log("Migrating funding...")
        rows = self.fetch_all("SELECT * FROM Wo_Funding ORDER BY id ASC")
        batch = []
        for r in rows:
            batch.append((
                r['id'], r.get('user_id'), safe_str(r.get('title', '')),
                safe_str(r.get('description', '')), safe_str(r.get('image', '')),
                Decimal(str(r.get('amount', 0) or 0)),
                Decimal(str(r.get('raised', 0) or 0)),
                ts(r.get('time', 0)),
            ))
        self._insert_batch('funding', ['id', 'user_id', 'title', 'description', 'image',
                                        'goal_amount', 'raised_amount', 'created_at'], batch)

    # ── Notifications ──
    def migrate_notifications(self):
        total = self.count('Wo_Notifications')
        self.log(f"Migrating {total} notifications...")
        rows = self.fetch_all("SELECT * FROM Wo_Notifications ORDER BY id ASC")
        batch = []
        for r in rows:
            batch.append((
                r['id'], r.get('recipient_id'), r.get('notifier_id'),
                safe_str(r.get('type', '')), safe_str(r.get('type2', '')),
                r.get('post_id') or r.get('page_id') or r.get('group_id'),
                safe_str(r.get('text', '')),
                boolish(r.get('seen', 0)), ts(r.get('time', 0)),
            ))
        self._insert_batch('notifications', ['id', 'recipient_id', 'sender_id',
                                              'type', 'target_type', 'target_id',
                                              'text', 'is_read', 'created_at'], batch)

    # ── Stories ──
    def migrate_stories(self):
        self.log("Migrating stories...")
        rows = self.fetch_all("SELECT * FROM Wo_Story ORDER BY id ASC")
        batch = []
        for r in rows:
            batch.append((r['id'], r.get('user_id'), ts(r.get('time', 0)),
                         ts(r.get('expire', 0))))
        self._insert_batch('stories', ['id', 'user_id', 'created_at', 'expires_at'], batch)

    # ── Config ──
    def migrate_config(self):
        self.log("Migrating config...")
        rows = self.fetch_all("SELECT * FROM Wo_Config")
        batch = []
        cid = 0
        for r in rows:
            cid += 1
            batch.append((cid, 'general', safe_str(r.get('name', '')),
                         safe_str(r.get('value', ''))))
        self._insert_batch('site_config', ['id', 'category', 'key', 'value'], batch)

    # ── Translations ──
    def migrate_translations(self):
        self.log("Migrating translations...")
        try:
            rows = self.fetch_all("SELECT * FROM Wo_Langs LIMIT 1")
            if not rows:
                return
            # Wo_Langs has columns as keys
            cols = [k for k in rows[0].keys() if k not in ('id', 'lang_name')]
            all_rows = self.fetch_all("SELECT * FROM Wo_Langs ORDER BY id ASC")
            batch = []
            tid = 0
            for r in all_rows:
                lang = safe_str(r.get('lang_name', 'english'))
                for col in cols:
                    val = safe_str(r.get(col, ''))
                    if val:
                        tid += 1
                        batch.append((tid, lang, col, val))
            self._insert_batch('translations', ['id', 'lang', 'key', 'value'], batch)
        except Exception as e:
            self.log(f"  Translations skipped: {e}")

    # ── Movies ──
    def migrate_movies(self):
        self.log("Migrating movies...")
        rows = self.fetch_all("SELECT * FROM Wo_Movies ORDER BY id ASC")
        batch = []
        for r in rows:
            batch.append((
                r['id'], r.get('user_id'), safe_str(r.get('name', '')),
                safe_str(r.get('cover', '')), safe_str(r.get('url', '')),
                safe_str(r.get('iframe', '')), safe_str(r.get('description', '')),
                safe_str(r.get('genre', '')), safe_str(r.get('country', '')),
                safe_str(r.get('stars', '')), safe_str(r.get('producer', '')),
                r.get('release', None), safe_str(r.get('duration', '')),
                safe_str(r.get('quality', '')),
                int(r.get('views', 0) or 0), boolish(r.get('active', 1)),
            ))
        self._insert_batch('movies', [
            'id', 'user_id', 'name', 'cover', 'video_url', 'iframe_url',
            'description', 'genre', 'country', 'stars', 'producer',
            'release_year', 'duration', 'quality', 'view_count', 'is_approved',
        ], batch)

    # ── Games ──
    def migrate_games(self):
        self.log("Migrating games...")
        rows = self.fetch_all("SELECT * FROM Wo_Games ORDER BY id ASC")
        batch = []
        for r in rows:
            batch.append((r['id'], safe_str(r.get('name', '')),
                         safe_str(r.get('img', '')), safe_str(r.get('url', '')),
                         boolish(r.get('active', 1)), 0))
        self._insert_batch('games', ['id', 'name', 'avatar', 'link', 'active', 'player_count'], batch)

    # ── Hashtags ──
    def migrate_hashtags(self):
        self.log("Migrating hashtags...")
        rows = self.fetch_all("SELECT * FROM Wo_Hashtags ORDER BY id ASC")
        batch = [(r['id'], safe_str(r.get('tag', '')),
                  int(r.get('count', 0) or 0), False, ts(r.get('last_trend_time', 0)))
                 for r in rows]
        self._insert_batch('hashtags', ['id', 'tag', 'use_count', 'is_trending', 'created_at'], batch)

    # ── Event Responses (Wo_Egoing + Wo_Einterested + Wo_Einvited) ──
    def migrate_event_responses(self):
        self.log("Migrating event responses...")
        batch = []
        rid = 0
        for table, response in [('Wo_Egoing', 'going'), ('Wo_Einterested', 'interested'), ('Wo_Einvited', 'invited')]:
            try:
                rows = self.fetch_all(f"SELECT * FROM {table}")
                for r in rows:
                    rid += 1
                    batch.append((rid, r.get('event_id'), r.get('user_id'), response))
            except Exception:
                continue
        self._insert_batch('event_responses', ['id', 'event_id', 'user_id', 'response'], batch)

    # ── Page Admins ──
    def migrate_page_admins(self):
        self.log("Migrating page admins...")
        try:
            rows = self.fetch_all("SELECT * FROM Wo_PageAdmins ORDER BY id ASC")
            batch = [(r['id'], r.get('page_id'), r.get('user_id'), 'admin') for r in rows]
            self._insert_batch('page_admins', ['id', 'page_id', 'user_id', 'role'], batch)
        except Exception:
            pass

    # ── Page Ratings ──
    def migrate_page_ratings(self):
        self.log("Migrating page ratings...")
        try:
            rows = self.fetch_all("SELECT * FROM Wo_PageRating ORDER BY id ASC")
            batch = [(r['id'], r.get('page_id'), r.get('user_id'),
                      int(r.get('value', 0) or 0), safe_str(r.get('review', '')),
                      ts(r.get('time', 0))) for r in rows]
            self._insert_batch('page_ratings', ['id', 'page_id', 'user_id', 'rating', 'review', 'created_at'], batch)
        except Exception:
            pass

    # ── Mutes (Wo_Mute + Wo_Mute_Story) ──
    def migrate_mutes(self):
        self.log("Migrating mutes...")
        batch = []
        mid = 0
        for table, mtype in [('Wo_Mute', 'user'), ('Wo_Mute_Story', 'story')]:
            try:
                rows = self.fetch_all(f"SELECT * FROM {table}")
                for r in rows:
                    mid += 1
                    batch.append((mid, r.get('user_id'), r.get('mute_id') or r.get('story_user_id'),
                                 mtype, ts(r.get('time', 0))))
            except Exception:
                continue
        self._insert_batch('mutes', ['id', 'user_id', 'target_id', 'mute_type', 'created_at'], batch)

    # ── Pokes ──
    def migrate_pokes(self):
        self.log("Migrating pokes...")
        try:
            rows = self.fetch_all("SELECT * FROM Wo_Pokes ORDER BY id ASC")
            batch = [(r['id'], r.get('user_id'), r.get('poke_to'),
                      ts(r.get('time', 0))) for r in rows]
            self._insert_batch('pokes', ['id', 'user_id', 'poked_user_id', 'created_at'], batch)
        except Exception:
            pass

    # ── Saved Posts ──
    def migrate_saved_posts(self):
        self.log("Migrating saved posts...")
        try:
            rows = self.fetch_all("SELECT * FROM Wo_SavedPosts ORDER BY id ASC")
            batch = [(r['id'], r.get('user_id'), r.get('post_id'),
                      ts(r.get('time', 0))) for r in rows]
            self._insert_batch('saved_posts', ['id', 'user_id', 'post_id', 'created_at'], batch)
        except Exception:
            pass

    # ── Hidden Posts ──
    def migrate_hidden_posts(self):
        self.log("Migrating hidden posts...")
        try:
            rows = self.fetch_all("SELECT * FROM Wo_HiddenPosts ORDER BY id ASC")
            batch = [(r['id'], r.get('user_id'), r.get('post_id')) for r in rows]
            self._insert_batch('hidden_posts', ['id', 'user_id', 'post_id'], batch)
        except Exception:
            pass

    # ── Story Media ──
    def migrate_story_media(self):
        self.log("Migrating story media...")
        try:
            rows = self.fetch_all("SELECT * FROM Wo_Story_Media ORDER BY id ASC")
            batch = [(r['id'], r.get('story_id'), safe_str(r.get('type', 'image')),
                      safe_str(r.get('filename', '')), safe_str(r.get('text', '')),
                      ts(r.get('time', 0))) for r in rows]
            self._insert_batch('story_media', ['id', 'story_id', 'media_type', 'media_url',
                                                'description', 'created_at'], batch)
        except Exception:
            pass

    # ── Story Views ──
    def migrate_story_views(self):
        self.log("Migrating story views...")
        try:
            rows = self.fetch_all("SELECT * FROM Wo_Story_Seen ORDER BY id ASC")
            batch = [(r['id'], r.get('story_media_id'), r.get('user_id'),
                      ts(r.get('time', 0))) for r in rows]
            self._insert_batch('story_views', ['id', 'story_media_id', 'user_id', 'viewed_at'], batch)
        except Exception:
            pass

    # ── Offers ──
    def migrate_offers(self):
        self.log("Migrating offers...")
        try:
            rows = self.fetch_all("SELECT * FROM Wo_Offers ORDER BY id ASC")
            batch = []
            for r in rows:
                batch.append((
                    r['id'], r.get('user_id'), safe_str(r.get('title', '')),
                    safe_str(r.get('description', '')), safe_str(r.get('image', '')),
                    Decimal(str(r.get('price', 0) or 0)),
                    Decimal(str(r.get('discount_price', 0) or 0)),
                    safe_str(r.get('currency', 'USD')),
                    ts(r.get('expire_date', 0)), ts(r.get('time', 0)),
                ))
            self._insert_batch('offers', ['id', 'user_id', 'title', 'description', 'image',
                                           'price', 'discount_price', 'currency',
                                           'expires_at', 'created_at'], batch)
        except Exception:
            pass

    # ── Payment Transactions ──
    def migrate_payments(self):
        self.log("Migrating payment transactions...")
        try:
            rows = self.fetch_all("SELECT * FROM Wo_Payment_Transactions ORDER BY id ASC")
            batch = []
            for r in rows:
                batch.append((
                    r['id'], r.get('user_id'),
                    Decimal(str(r.get('amount', 0) or 0)),
                    safe_str(r.get('currency', 'USD')),
                    safe_str(r.get('via', '')),
                    safe_str(r.get('kind', '')),
                    'completed',
                    ts(r.get('time', 0)),
                ))
            self._insert_batch('payment_transactions', ['id', 'user_id', 'amount', 'currency',
                                                         'provider', 'type', 'status', 'created_at'], batch)
        except Exception:
            pass

    # ── User Ads ──
    def migrate_ads(self):
        self.log("Migrating user ads...")
        try:
            rows = self.fetch_all("SELECT * FROM Wo_UserAds ORDER BY id ASC")
            batch = []
            for r in rows:
                batch.append((
                    r['id'], r.get('user_id'), safe_str(r.get('name', '')),
                    safe_str(r.get('headline', '')), safe_str(r.get('description', '')),
                    safe_str(r.get('image', '')), safe_str(r.get('url', '')),
                    safe_str(r.get('ad_type', 'post')),
                    Decimal(str(r.get('budget', 0) or 0)),
                    safe_str(r.get('bid_type', 'views')),
                    'active' if boolish(r.get('active', 0)) else 'paused',
                    int(r.get('impressions', 0) or 0),
                    int(r.get('clicks', 0) or 0),
                    ts(r.get('time', 0)),
                ))
            self._insert_batch('user_ads', ['id', 'user_id', 'name', 'headline', 'description',
                                             'image', 'url', 'ad_type', 'budget', 'bid_type',
                                             'status', 'impressions', 'clicks', 'created_at'], batch)
        except Exception:
            pass

    # ── Reports ──
    def migrate_reports(self):
        self.log("Migrating reports...")
        try:
            rows = self.fetch_all("SELECT * FROM Wo_Reports ORDER BY id ASC")
            batch = []
            for r in rows:
                target_type = 'post'
                target_id = r.get('post_id')
                if r.get('profile_id'):
                    target_type = 'user'
                    target_id = r['profile_id']
                elif r.get('page_id'):
                    target_type = 'page'
                    target_id = r['page_id']
                elif r.get('group_id'):
                    target_type = 'group'
                    target_id = r['group_id']
                elif r.get('comment_id'):
                    target_type = 'comment'
                    target_id = r['comment_id']
                batch.append((
                    r['id'], r.get('user_id'), target_type, target_id,
                    safe_str(r.get('reason', '')), safe_str(r.get('text', '')),
                    'pending', ts(r.get('time', 0)),
                ))
            self._insert_batch('reports', ['id', 'reporter_id', 'target_type', 'target_id',
                                            'reason', 'description', 'status', 'created_at'], batch)
        except Exception:
            pass

    # ── Announcements ──
    def migrate_announcements(self):
        self.log("Migrating announcements...")
        try:
            rows = self.fetch_all("SELECT * FROM Wo_Announcement ORDER BY id ASC")
            batch = [(r['id'], safe_str(r.get('text', '')), safe_str(r.get('text_en', '')),
                      boolish(r.get('active', 1)), ts(r.get('time', 0))) for r in rows]
            self._insert_batch('announcements', ['id', 'text', 'text_en', 'is_active', 'created_at'], batch)
        except Exception:
            pass

    # ── Verification Requests ──
    def migrate_verification_requests(self):
        self.log("Migrating verification requests...")
        try:
            rows = self.fetch_all("SELECT * FROM Wo_Verification ORDER BY id ASC")
            batch = []
            for r in rows:
                batch.append((
                    r['id'], r.get('user_id'), safe_str(r.get('full_name', '')),
                    safe_str(r.get('message', '')), safe_str(r.get('image', '')),
                    'pending', ts(r.get('time', 0)),
                ))
            self._insert_batch('verification_requests', ['id', 'user_id', 'full_name',
                                                          'message', 'document_url',
                                                          'status', 'created_at'], batch)
        except Exception:
            pass

    # ── Activities ──
    def migrate_activities(self):
        self.log("Migrating activities...")
        try:
            rows = self.fetch_all("SELECT * FROM Wo_Activities ORDER BY id ASC")
            batch = [(r['id'], r.get('user_id'), safe_str(r.get('action', '')),
                      safe_str(r.get('full_action', '')),
                      ts(r.get('time', 0))) for r in rows]
            self._insert_batch('activities', ['id', 'user_id', 'action', 'description', 'created_at'], batch)
        except Exception:
            pass

    # ── Calls (Wo_VideoCalles + Wo_AudioCalls) ──
    def migrate_calls(self):
        self.log("Migrating calls...")
        batch = []
        cid = 0
        for table, ctype in [('Wo_VideoCalles', 'video'), ('Wo_AudioCalls', 'audio')]:
            try:
                rows = self.fetch_all(f"SELECT * FROM {table}")
                for r in rows:
                    cid += 1
                    batch.append((
                        cid, r.get('from_id'), r.get('to_id'), ctype,
                        safe_str(r.get('status', 'ended')),
                        ts(r.get('time', 0)),
                    ))
            except Exception:
                continue
        self._insert_batch('calls', ['id', 'caller_id', 'callee_id', 'call_type',
                                      'status', 'created_at'], batch)

    # ── Stickers ──
    def migrate_stickers(self):
        self.log("Migrating stickers...")
        try:
            rows = self.fetch_all("SELECT * FROM Wo_Stickers ORDER BY id ASC")
            batch = [(r['id'], safe_str(r.get('name', '')), int(r.get('count', 0) or 0),
                      Decimal(str(r.get('price', 0) or 0)),
                      boolish(r.get('active', 1))) for r in rows]
            self._insert_batch('sticker_packs', ['id', 'name', 'sticker_count', 'price', 'is_active'], batch)
        except Exception:
            pass

    # ── Gifts ──
    def migrate_gifts(self):
        self.log("Migrating gifts...")
        try:
            rows = self.fetch_all("SELECT * FROM Wo_Gifts ORDER BY id ASC")
            batch = [(r['id'], safe_str(r.get('name', '')), safe_str(r.get('img', '')),
                      Decimal(str(r.get('price', 0) or 0)),
                      boolish(r.get('active', 1))) for r in rows]
            self._insert_batch('gifts', ['id', 'name', 'image', 'price', 'is_active'], batch)
        except Exception:
            pass
        try:
            rows = self.fetch_all("SELECT * FROM Wo_User_Gifts ORDER BY id ASC")
            batch = [(r['id'], r.get('from_id'), r.get('to_id'), r.get('gift_id'),
                      ts(r.get('time', 0))) for r in rows]
            self._insert_batch('user_gifts', ['id', 'sender_id', 'recipient_id', 'gift_id', 'created_at'], batch)
        except Exception:
            pass

    # ── Banned IPs ──
    def migrate_banned_ips(self):
        self.log("Migrating banned IPs...")
        try:
            rows = self.fetch_all("SELECT * FROM Wo_Banned_Ip ORDER BY id ASC")
            batch = [(r['id'], safe_str(r.get('ip', '')),
                      safe_str(r.get('reason', '')),
                      ts(r.get('time', 0))) for r in rows]
            self._insert_batch('banned_ips', ['id', 'ip_address', 'reason', 'created_at'], batch)
        except Exception:
            pass

    # ── Login Attempts ──
    def migrate_login_attempts(self):
        self.log("Migrating login attempts...")
        try:
            rows = self.fetch_all("SELECT * FROM Wo_Bad_Login ORDER BY id ASC")
            batch = [(r['id'], safe_str(r.get('ip_address', '')),
                      safe_str(r.get('email', '')),
                      ts(r.get('time', 0))) for r in rows]
            self._insert_batch('login_attempts', ['id', 'ip_address', 'email', 'attempted_at'], batch)
        except Exception:
            pass

    # ── User Experience ──
    def migrate_user_experience(self):
        self.log("Migrating user experience...")
        try:
            rows = self.fetch_all("SELECT * FROM Wo_UserExperience ORDER BY id ASC")
            batch = []
            for r in rows:
                batch.append((
                    r['id'], r.get('user_id'), safe_str(r.get('title', '')),
                    safe_str(r.get('company', '')), safe_str(r.get('location', '')),
                    safe_str(r.get('description', '')),
                    safe_str(r.get('from', '')), safe_str(r.get('to', '')),
                    boolish(r.get('currently_working', 0)),
                ))
            self._insert_batch('user_experience', ['id', 'user_id', 'title', 'company',
                                                    'location', 'description', 'start_date',
                                                    'end_date', 'is_current'], batch)
        except Exception:
            pass

    # ── User Certifications ──
    def migrate_user_certifications(self):
        self.log("Migrating user certifications...")
        try:
            rows = self.fetch_all("SELECT * FROM Wo_UserCertification ORDER BY id ASC")
            batch = []
            for r in rows:
                batch.append((
                    r['id'], r.get('user_id'), safe_str(r.get('name', '')),
                    safe_str(r.get('authority', '')), safe_str(r.get('url', '')),
                    safe_str(r.get('from', '')), safe_str(r.get('to', '')),
                ))
            self._insert_batch('user_certifications', ['id', 'user_id', 'name', 'authority',
                                                        'url', 'start_date', 'end_date'], batch)
        except Exception:
            pass

    # ── User Skills ──
    def migrate_user_skills(self):
        self.log("Migrating user skills...")
        try:
            rows = self.fetch_all("SELECT * FROM Wo_UserSkills ORDER BY id ASC")
            batch = [(r['id'], r.get('user_id'), safe_str(r.get('skill', ''))) for r in rows]
            self._insert_batch('user_skills', ['id', 'user_id', 'name'], batch)
        except Exception:
            pass

    # ── Recent Searches ──
    def migrate_recent_searches(self):
        self.log("Migrating recent searches...")
        try:
            rows = self.fetch_all("SELECT * FROM Wo_RecentSearches ORDER BY id ASC")
            batch = [(r['id'], r.get('user_id'), safe_str(r.get('search', '')),
                      ts(r.get('time', 0))) for r in rows]
            self._insert_batch('recent_searches', ['id', 'user_id', 'query', 'created_at'], batch)
        except Exception:
            pass

    # ── Colored Post Templates ──
    def migrate_colored_post_templates(self):
        self.log("Migrating colored post templates...")
        try:
            rows = self.fetch_all("SELECT * FROM Wo_Colored_Posts ORDER BY id ASC")
            batch = []
            for r in rows:
                batch.append((
                    r['id'], safe_str(r.get('color_1', '')), safe_str(r.get('color_2', '')),
                    safe_str(r.get('text_color', '#ffffff')),
                    safe_str(r.get('image', '')), boolish(r.get('active', 1)),
                ))
            self._insert_batch('colored_post_templates', ['id', 'color_1', 'color_2',
                                                           'text_color', 'image', 'is_active'], batch)
        except Exception:
            pass

    # ── Reaction Types ──
    def migrate_reaction_types(self):
        self.log("Migrating reaction types...")
        try:
            rows = self.fetch_all("SELECT * FROM Wo_Reactions_Types ORDER BY id ASC")
            batch = [(r['id'], safe_str(r.get('name', '')), safe_str(r.get('icon', '')),
                      boolish(r.get('is_active', 1))) for r in rows]
            self._insert_batch('reaction_types', ['id', 'name', 'icon', 'is_active'], batch)
        except Exception:
            pass

    # ── Invitation Links ──
    def migrate_invitation_links(self):
        self.log("Migrating invitation links...")
        try:
            rows = self.fetch_all("SELECT * FROM Wo_Invitation_Links ORDER BY id ASC")
            batch = [(r['id'], r.get('user_id'), safe_str(r.get('code', '')),
                      int(r.get('uses', 0) or 0), ts(r.get('time', 0))) for r in rows]
            self._insert_batch('invitation_links', ['id', 'user_id', 'code', 'uses', 'created_at'], batch)
        except Exception:
            pass

    # ── Custom Pages ──
    def migrate_custom_pages(self):
        self.log("Migrating custom pages...")
        try:
            rows = self.fetch_all("SELECT * FROM Wo_CustomPages ORDER BY id ASC")
            batch = [(r['id'], safe_str(r.get('page_name', '')),
                      safe_str(r.get('page_title', '')),
                      safe_str(r.get('page_content', '')),
                      boolish(r.get('active', 1))) for r in rows]
            self._insert_batch('custom_pages', ['id', 'slug', 'title', 'content', 'is_active'], batch)
        except Exception:
            pass

    # ── Email Templates ──
    def migrate_email_templates(self):
        self.log("Migrating email templates...")
        try:
            rows = self.fetch_all("SELECT * FROM Wo_HTML_Emails ORDER BY id ASC")
            batch = [(r['id'], safe_str(r.get('name', '')),
                      safe_str(r.get('subject', '')),
                      safe_str(r.get('body', ''))) for r in rows]
            self._insert_batch('email_templates', ['id', 'name', 'subject', 'body'], batch)
        except Exception:
            pass

    # ── Polls ──
    def migrate_polls(self):
        self.log("Migrating polls...")
        try:
            rows = self.fetch_all("SELECT * FROM Wo_Polls ORDER BY id ASC")
            batch = []
            for r in rows:
                options = safe_json(r.get('options', '[]'))
                if isinstance(options, str):
                    try:
                        options = json.loads(options)
                    except Exception:
                        options = []
                batch.append((r['id'], r.get('post_id'), json.dumps(options)))
            self._insert_batch('polls', ['id', 'post_id', 'options'], batch)
        except Exception:
            pass
        try:
            rows = self.fetch_all("SELECT * FROM Wo_Votes ORDER BY id ASC")
            batch = [(r['id'], r.get('poll_id'), r.get('user_id'),
                      int(r.get('option_index', 0) or 0)) for r in rows]
            self._insert_batch('poll_votes', ['id', 'poll_id', 'user_id', 'option_index'], batch)
        except Exception:
            pass

    # ── Orders (Wo_UserOrders + Wo_Purchases) ──
    def migrate_orders(self):
        self.log("Migrating orders...")
        try:
            rows = self.fetch_all("SELECT * FROM Wo_UserOrders ORDER BY id ASC")
            batch = []
            for r in rows:
                batch.append((
                    r['id'], r.get('user_id'), r.get('product_id'), r.get('seller_id'),
                    int(r.get('quantity', 1) or 1),
                    Decimal(str(r.get('price', 0) or 0)),
                    safe_str(r.get('status', 'pending')),
                    ts(r.get('time', 0)),
                ))
            self._insert_batch('orders', ['id', 'buyer_id', 'product_id', 'seller_id',
                                           'quantity', 'total_price', 'status', 'created_at'], batch)
        except Exception:
            pass

    # ── Job Applications ──
    def migrate_job_applications(self):
        self.log("Migrating job applications...")
        try:
            rows = self.fetch_all("SELECT * FROM Wo_Job_Apply ORDER BY id ASC")
            batch = []
            for r in rows:
                answers = safe_json(r.get('answers', '{}'))
                batch.append((
                    r['id'], r.get('job_id'), r.get('user_id'),
                    json.dumps(answers), 'pending',
                    ts(r.get('time', 0)),
                ))
            self._insert_batch('job_applications', ['id', 'job_id', 'user_id',
                                                     'answers', 'status', 'created_at'], batch)
        except Exception:
            pass

    # ── Funding Donations ──
    def migrate_funding_donations(self):
        self.log("Migrating funding donations...")
        try:
            rows = self.fetch_all("SELECT * FROM Wo_Funding_Raise ORDER BY id ASC")
            batch = [(r['id'], r.get('funding_id'), r.get('user_id'),
                      Decimal(str(r.get('amount', 0) or 0)),
                      ts(r.get('time', 0))) for r in rows]
            self._insert_batch('funding_donations', ['id', 'funding_id', 'user_id',
                                                      'amount', 'created_at'], batch)
        except Exception:
            pass

    # ── Product Reviews ──
    def migrate_product_reviews(self):
        self.log("Migrating product reviews...")
        try:
            rows = self.fetch_all("SELECT * FROM Wo_ProductReview ORDER BY id ASC")
            batch = [(r['id'], r.get('product_id'), r.get('user_id'),
                      int(r.get('rating', 0) or 0), safe_str(r.get('text', '')),
                      ts(r.get('time', 0))) for r in rows]
            self._insert_batch('product_reviews', ['id', 'product_id', 'user_id',
                                                    'rating', 'text', 'created_at'], batch)
        except Exception:
            pass

    # ── Profile Fields ──
    def migrate_profile_fields(self):
        self.log("Migrating profile fields...")
        try:
            rows = self.fetch_all("SELECT * FROM Wo_ProfileFields ORDER BY id ASC")
            batch = [(r['id'], safe_str(r.get('name', '')), safe_str(r.get('description', '')),
                      safe_str(r.get('type', 'text')),
                      boolish(r.get('active', 1))) for r in rows]
            self._insert_batch('profile_fields', ['id', 'name', 'description', 'field_type',
                                                   'is_active'], batch)
        except Exception:
            pass
        try:
            rows = self.fetch_all("SELECT * FROM Wo_UserFields ORDER BY id ASC")
            batch = [(r['id'], r.get('user_id'), r.get('field_id'),
                      safe_str(r.get('value', ''))) for r in rows]
            self._insert_batch('user_field_values', ['id', 'user_id', 'field_id', 'value'], batch)
        except Exception:
            pass

    # ── Uploaded Media ──
    def migrate_uploaded_media(self):
        self.log("Migrating uploaded media...")
        try:
            rows = self.fetch_all("SELECT * FROM Wo_UploadedMedia ORDER BY id ASC")
            batch = [(r['id'], r.get('user_id'), safe_str(r.get('file_url', '')),
                      safe_str(r.get('file_type', '')),
                      int(r.get('file_size', 0) or 0),
                      safe_str(r.get('mime_type', '')),
                      ts(r.get('time', 0))) for r in rows]
            self._insert_batch('uploaded_media', ['id', 'user_id', 'file_url', 'file_type',
                                                   'file_size', 'mime_type', 'created_at'], batch)
        except Exception:
            pass

    # ── Sequence Reset ──
    def reset_sequences(self):
        self.log("Resetting PostgreSQL sequences...")
        cur = self.pg.cursor()
        cur.execute("""
            SELECT tablename FROM pg_tables WHERE schemaname = 'public'
        """)
        tables = [r[0] for r in cur.fetchall()]
        for table in tables:
            try:
                cur.execute(f"SELECT setval(pg_get_serial_sequence('{table}', 'id'), COALESCE(MAX(id), 1)) FROM {table}")
            except Exception:
                self.pg.rollback()
                continue
        self.pg.commit()
        cur.close()
        self.log("  Sequences reset complete")

    # ── Verification ──
    def verify_migration(self):
        self.log("Verifying migration counts...")
        checks = [
            ('Wo_Users', 'users'), ('Wo_Posts', 'posts'),
            ('Wo_Followers', 'follows'), ('Wo_Pages', 'pages'),
            ('Wo_Groups', 'groups'), ('Wo_Notifications', 'notifications'),
        ]
        all_ok = True
        for src, dst in checks:
            try:
                src_count = self.count(src)
                pg_cur = self.pg.cursor()
                pg_cur.execute(f"SELECT COUNT(*) FROM {dst}")
                dst_count = pg_cur.fetchone()[0]
                pg_cur.close()
                status = '✓' if dst_count >= src_count else '⚠'
                if dst_count < src_count:
                    all_ok = False
                self.log(f"  {status} {src} ({src_count}) → {dst} ({dst_count})")
            except Exception:
                self.log(f"  ⚠ Could not verify {src} → {dst}")
        if all_ok:
            self.log("  All core tables verified")
        else:
            self.log("  WARNING: Some tables have count mismatches")


def main():
    parser = argparse.ArgumentParser(description='WoWonder MySQL → PostgreSQL Migrator')
    parser.add_argument('--mysql-host', default='127.0.0.1')
    parser.add_argument('--mysql-port', type=int, default=3306)
    parser.add_argument('--mysql-db', required=True)
    parser.add_argument('--mysql-user', default='root')
    parser.add_argument('--mysql-pass', default='')
    parser.add_argument('--pg-host', default='127.0.0.1')
    parser.add_argument('--pg-port', type=int, default=5432)
    parser.add_argument('--pg-db', default='wowonder')
    parser.add_argument('--pg-user', default='postgres')
    parser.add_argument('--pg-pass', default='postgres')
    parser.add_argument('--batch-size', type=int, default=1000)
    args = parser.parse_args()

    mysql_cfg = {
        'host': args.mysql_host, 'port': args.mysql_port,
        'database': args.mysql_db, 'user': args.mysql_user,
        'password': args.mysql_pass,
    }
    pg_cfg = {
        'host': args.pg_host, 'port': args.pg_port,
        'dbname': args.pg_db, 'user': args.pg_user,
        'password': args.pg_pass,
    }

    migrator = Migrator(mysql_cfg, pg_cfg, args.batch_size)
    migrator.run()


if __name__ == '__main__':
    main()
