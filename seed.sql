BEGIN;

-- ── 10 Fake Users (password = same as admin user) ─────────────────────────
INSERT INTO users (username, email, password_hash, first_name, last_name, avatar, cover, about, gender, birthday, city, working, school, is_active, email_verified, is_verified)
VALUES
('sophia.martinez','sophia@example.com','$argon2id$v=19$m=19456,t=2,p=1$ZnAmvQCuH7RxtEcogWR85Q$yDHvBQMgG+w7dCjty6CisXmQsVZNpW0NzRsZIumjroA','Sophia','Martinez','https://randomuser.me/api/portraits/women/44.jpg','https://images.unsplash.com/photo-1506905925346-21bda4d32df4?w=1200','Photographer and travel enthusiast. Exploring the world one city at a time.','female','1995-03-15','Barcelona','Freelance Photographer','UAB Barcelona',true,true,true),
('alex.chen','alex@example.com','$argon2id$v=19$m=19456,t=2,p=1$ZnAmvQCuH7RxtEcogWR85Q$yDHvBQMgG+w7dCjty6CisXmQsVZNpW0NzRsZIumjroA','Alex','Chen','https://randomuser.me/api/portraits/men/32.jpg','https://images.unsplash.com/photo-1517694712202-14dd9538aa97?w=1200','Full-stack developer. Open source contributor. Coffee addict.','male','1992-07-22','San Francisco','Senior Dev at TechCorp','Stanford University',true,true,true),
('luna.rossi','luna@example.com','$argon2id$v=19$m=19456,t=2,p=1$ZnAmvQCuH7RxtEcogWR85Q$yDHvBQMgG+w7dCjty6CisXmQsVZNpW0NzRsZIumjroA','Luna','Rossi','https://randomuser.me/api/portraits/women/68.jpg','https://images.unsplash.com/photo-1470071459604-3b5ec3a7fe05?w=1200','Digital artist and illustrator. Turning imagination into pixels.','female','1998-11-08','Milan','Digital Artist at CreativeStudio','Politecnico di Milano',true,true,false),
('marcus.johnson','marcus@example.com','$argon2id$v=19$m=19456,t=2,p=1$ZnAmvQCuH7RxtEcogWR85Q$yDHvBQMgG+w7dCjty6CisXmQsVZNpW0NzRsZIumjroA','Marcus','Johnson','https://randomuser.me/api/portraits/men/75.jpg','https://images.unsplash.com/photo-1461896836934-bd45ba8fd148?w=1200','Fitness coach and nutritionist. Helping people become their best version.','male','1990-01-30','Miami','Fitness Coach','University of Miami',true,true,false),
('emma.williams','emma@example.com','$argon2id$v=19$m=19456,t=2,p=1$ZnAmvQCuH7RxtEcogWR85Q$yDHvBQMgG+w7dCjty6CisXmQsVZNpW0NzRsZIumjroA','Emma','Williams','https://randomuser.me/api/portraits/women/22.jpg','https://images.unsplash.com/photo-1504674900247-0877df9cc836?w=1200','Food blogger and recipe creator. Life is too short for bad food.','female','1993-06-12','London','Food Blogger','Le Cordon Bleu',true,true,true),
('kai.nakamura','kai@example.com','$argon2id$v=19$m=19456,t=2,p=1$ZnAmvQCuH7RxtEcogWR85Q$yDHvBQMgG+w7dCjty6CisXmQsVZNpW0NzRsZIumjroA','Kai','Nakamura','https://randomuser.me/api/portraits/men/52.jpg','https://images.unsplash.com/photo-1519681393784-d120267933ba?w=1200','Music producer and DJ. Making beats that move your soul.','male','1996-09-25','Tokyo','Music Producer','Berklee College of Music',true,true,false),
('olivia.brown','olivia@example.com','$argon2id$v=19$m=19456,t=2,p=1$ZnAmvQCuH7RxtEcogWR85Q$yDHvBQMgG+w7dCjty6CisXmQsVZNpW0NzRsZIumjroA','Olivia','Brown','https://randomuser.me/api/portraits/women/33.jpg','https://images.unsplash.com/photo-1507003211169-0a1dd7228f2d?w=1200','UX Designer. Creating beautiful digital experiences.','female','1994-12-03','Berlin','Lead UX Designer at DesignLab','Berlin Design Academy',true,true,false),
('diego.garcia','diego@example.com','$argon2id$v=19$m=19456,t=2,p=1$ZnAmvQCuH7RxtEcogWR85Q$yDHvBQMgG+w7dCjty6CisXmQsVZNpW0NzRsZIumjroA','Diego','Garcia','https://randomuser.me/api/portraits/men/67.jpg','https://images.unsplash.com/photo-1542281286-9e0a16bb7366?w=1200','Startup founder and tech entrepreneur. Building the future.','male','1991-04-18','Mexico City','CEO at InnovateMX','ITESM',true,true,true),
('nina.petrova','nina@example.com','$argon2id$v=19$m=19456,t=2,p=1$ZnAmvQCuH7RxtEcogWR85Q$yDHvBQMgG+w7dCjty6CisXmQsVZNpW0NzRsZIumjroA','Nina','Petrova','https://randomuser.me/api/portraits/women/56.jpg','https://images.unsplash.com/photo-1518837695005-2083093ee35b?w=1200','Marine biologist and ocean conservation advocate.','female','1997-02-14','Moscow','Marine Biologist at OceanLab','Moscow State University',true,true,false),
('james.taylor','james@example.com','$argon2id$v=19$m=19456,t=2,p=1$ZnAmvQCuH7RxtEcogWR85Q$yDHvBQMgG+w7dCjty6CisXmQsVZNpW0NzRsZIumjroA','James','Taylor','https://randomuser.me/api/portraits/men/85.jpg','https://images.unsplash.com/photo-1475924156734-496f6cac6ec1?w=1200','Adventure filmmaker and storyteller. Life begins outside your comfort zone.','male','1989-08-07','Sydney','Filmmaker at WildLens','AFTRS',true,true,true);

-- Record IDs for reference
DO $$
DECLARE
  u_sophia BIGINT; u_alex BIGINT; u_luna BIGINT; u_marcus BIGINT; u_emma BIGINT;
  u_kai BIGINT; u_olivia BIGINT; u_diego BIGINT; u_nina BIGINT; u_james BIGINT;
  p_id BIGINT; s_id BIGINT; c_id BIGINT;
BEGIN
  SELECT id INTO u_sophia FROM users WHERE username='sophia.martinez';
  SELECT id INTO u_alex FROM users WHERE username='alex.chen';
  SELECT id INTO u_luna FROM users WHERE username='luna.rossi';
  SELECT id INTO u_marcus FROM users WHERE username='marcus.johnson';
  SELECT id INTO u_emma FROM users WHERE username='emma.williams';
  SELECT id INTO u_kai FROM users WHERE username='kai.nakamura';
  SELECT id INTO u_olivia FROM users WHERE username='olivia.brown';
  SELECT id INTO u_diego FROM users WHERE username='diego.garcia';
  SELECT id INTO u_nina FROM users WHERE username='nina.petrova';
  SELECT id INTO u_james FROM users WHERE username='james.taylor';

  -- ── Follows (everyone follows admin=2, cross-follows between users) ──────
  INSERT INTO follows (follower_id, following_id, status) VALUES
  (u_sophia, 2, 'active'),(u_alex, 2, 'active'),(u_luna, 2, 'active'),
  (u_marcus, 2, 'active'),(u_emma, 2, 'active'),(u_kai, 2, 'active'),
  (u_olivia, 2, 'active'),(u_diego, 2, 'active'),(u_nina, 2, 'active'),
  (u_james, 2, 'active'),
  -- Admin follows them back
  (2, u_sophia, 'active'),(2, u_alex, 'active'),(2, u_luna, 'active'),
  (2, u_marcus, 'active'),(2, u_emma, 'active'),(2, u_kai, 'active'),
  (2, u_olivia, 'active'),(2, u_diego, 'active'),(2, u_nina, 'active'),
  (2, u_james, 'active'),
  -- Cross-follows between fake users
  (u_sophia, u_alex, 'active'),(u_alex, u_sophia, 'active'),
  (u_sophia, u_luna, 'active'),(u_luna, u_sophia, 'active'),
  (u_sophia, u_emma, 'active'),(u_emma, u_sophia, 'active'),
  (u_alex, u_diego, 'active'),(u_diego, u_alex, 'active'),
  (u_alex, u_kai, 'active'),(u_kai, u_alex, 'active'),
  (u_luna, u_olivia, 'active'),(u_olivia, u_luna, 'active'),
  (u_marcus, u_james, 'active'),(u_james, u_marcus, 'active'),
  (u_marcus, u_nina, 'active'),(u_nina, u_marcus, 'active'),
  (u_emma, u_nina, 'active'),(u_nina, u_emma, 'active'),
  (u_kai, u_james, 'active'),(u_james, u_kai, 'active'),
  (u_olivia, u_diego, 'active'),(u_diego, u_olivia, 'active')
  ON CONFLICT DO NOTHING;

  -- ── Posts (3-4 per user, varied types and timestamps) ────────────────────
  -- Sophia - photographer
  INSERT INTO posts (user_id, content, post_type, media, feeling, created_at, like_count, comment_count) VALUES
  (u_sophia, 'Just captured this incredible sunset over the Mediterranean. The colors were absolutely magical today! #photography #sunset #barcelona', 'photo', '[{"url":"https://images.unsplash.com/photo-1507525428034-b723cf961d3e?w=800","type":"image"}]', 'happy', NOW() - interval '2 hours', 24, 3)
  RETURNING id INTO p_id;

  INSERT INTO posts (user_id, content, post_type, media, created_at, like_count, comment_count) VALUES
  (u_sophia, 'Behind the scenes of today''s photoshoot. Working with natural light is always a challenge but the results are worth it.', 'photo', '[{"url":"https://images.unsplash.com/photo-1452587925148-ce544e77e70d?w=800","type":"image"}]', NOW() - interval '1 day', 18, 2);

  INSERT INTO posts (user_id, content, post_type, created_at, like_count, comment_count) VALUES
  (u_sophia, 'Tip for aspiring photographers: The best camera is the one you have with you. Stop waiting for perfect equipment and start shooting!', 'text', NOW() - interval '3 days', 42, 5);

  -- Alex - developer
  INSERT INTO posts (user_id, content, post_type, created_at, like_count, comment_count) VALUES
  (u_alex, 'Just shipped a major feature using Rust + WebAssembly. The performance gains are insane - 40x faster than the JS implementation! #rustlang #webdev', 'text', NOW() - interval '1 hour', 35, 7);

  INSERT INTO posts (user_id, content, post_type, media, created_at, like_count, comment_count) VALUES
  (u_alex, 'My home office setup for 2026. Finally got the ultrawide monitor and it changed everything.', 'photo', '[{"url":"https://images.unsplash.com/photo-1593062096033-9a26b09da705?w=800","type":"image"}]', NOW() - interval '2 days', 28, 4);

  INSERT INTO posts (user_id, content, post_type, created_at, like_count, comment_count) VALUES
  (u_alex, 'Hot take: TypeScript is just Java for people who are too cool to admit they like Java.', 'text', NOW() - interval '5 days', 89, 23);

  -- Luna - artist
  INSERT INTO posts (user_id, content, post_type, media, feeling, created_at, like_count, comment_count) VALUES
  (u_luna, 'New digital painting completed! This one took about 40 hours. Swipe to see the process timelapse. #digitalart #illustration', 'photo', '[{"url":"https://images.unsplash.com/photo-1579783902614-a3fb3927b6a5?w=800","type":"image"},{"url":"https://images.unsplash.com/photo-1513364776144-60967b0f800f?w=800","type":"image"}]', 'proud', NOW() - interval '3 hours', 67, 12);

  INSERT INTO posts (user_id, content, post_type, media, created_at, like_count, comment_count) VALUES
  (u_luna, 'Experimenting with a new style today. What do you think? Should I keep going with this direction?', 'photo', '[{"url":"https://images.unsplash.com/photo-1541961017774-22349e4a1262?w=800","type":"image"}]', NOW() - interval '1 day', 45, 8);

  INSERT INTO posts (user_id, content, post_type, created_at, like_count, comment_count) VALUES
  (u_luna, 'Art is not what you see, but what you make others see. - Edgar Degas', 'text', NOW() - interval '4 days', 31, 2);

  -- Marcus - fitness
  INSERT INTO posts (user_id, content, post_type, media, feeling, created_at, like_count, comment_count) VALUES
  (u_marcus, 'Leg day is the best day! Here is my current routine that has been giving me incredible results. Drop your favorite leg exercises below!', 'photo', '[{"url":"https://images.unsplash.com/photo-1534438327276-14e5300c3a48?w=800","type":"image"}]', 'strong', NOW() - interval '4 hours', 52, 15);

  INSERT INTO posts (user_id, content, post_type, created_at, like_count, comment_count) VALUES
  (u_marcus, 'Nutrition tip: You cannot out-train a bad diet. Focus on whole foods, adequate protein (1.6-2.2g/kg), and proper hydration. Your body will thank you.', 'text', NOW() - interval '2 days', 38, 6);

  -- Emma - food blogger
  INSERT INTO posts (user_id, content, post_type, media, feeling, created_at, like_count, comment_count) VALUES
  (u_emma, 'Made this incredible homemade pasta from scratch today. The secret is in the resting time - let the dough relax for at least 30 minutes!', 'photo', '[{"url":"https://images.unsplash.com/photo-1473093295043-cdd812d0e601?w=800","type":"image"}]', 'happy', NOW() - interval '5 hours', 73, 18);

  INSERT INTO posts (user_id, content, post_type, media, created_at, like_count, comment_count) VALUES
  (u_emma, 'Food market finds in London. These artisan cheeses are absolutely divine!', 'photo', '[{"url":"https://images.unsplash.com/photo-1452195100486-9cc805987862?w=800","type":"image"},{"url":"https://images.unsplash.com/photo-1486297678162-eb2a19b0a32d?w=800","type":"image"}]', NOW() - interval '3 days', 29, 4);

  INSERT INTO posts (user_id, content, post_type, created_at, like_count, comment_count) VALUES
  (u_emma, 'Recipe drop! My grandmother''s secret tomato sauce recipe. Thread incoming...', 'text', NOW() - interval '6 days', 156, 34);

  -- Kai - music producer
  INSERT INTO posts (user_id, content, post_type, media, created_at, like_count, comment_count) VALUES
  (u_kai, 'New track dropping this Friday! Here is a sneak peek of the production session. #music #producer', 'photo', '[{"url":"https://images.unsplash.com/photo-1598488035139-bdbb2231ce04?w=800","type":"image"}]', NOW() - interval '6 hours', 41, 9);

  INSERT INTO posts (user_id, content, post_type, created_at, like_count, comment_count) VALUES
  (u_kai, 'Spending the whole weekend in the studio. Sometimes the best music comes from those marathon sessions where you lose track of time.', 'text', NOW() - interval '1 day', 22, 3);

  -- Olivia - UX designer
  INSERT INTO posts (user_id, content, post_type, media, created_at, like_count, comment_count) VALUES
  (u_olivia, 'Redesigned the onboarding flow for our app. Conversion rate went up 35%! A/B testing is everything. #ux #design', 'photo', '[{"url":"https://images.unsplash.com/photo-1586717791821-3f44a563fa4c?w=800","type":"image"}]', NOW() - interval '7 hours', 55, 11);

  INSERT INTO posts (user_id, content, post_type, created_at, like_count, comment_count) VALUES
  (u_olivia, 'Unpopular opinion: dark mode is not always the best choice. Sometimes light mode provides better readability and reduces eye strain in well-lit environments.', 'text', NOW() - interval '2 days', 94, 47);

  -- Diego - entrepreneur
  INSERT INTO posts (user_id, content, post_type, created_at, like_count, comment_count) VALUES
  (u_diego, 'Excited to announce that our startup just closed a $5M Series A round! Grateful for the amazing team and investors who believed in our vision.', 'text', NOW() - interval '8 hours', 182, 28);

  INSERT INTO posts (user_id, content, post_type, media, created_at, like_count, comment_count) VALUES
  (u_diego, 'Team retreat in Tulum. Building company culture is just as important as building the product.', 'photo', '[{"url":"https://images.unsplash.com/photo-1504384308090-c894fdcc538d?w=800","type":"image"}]', NOW() - interval '4 days', 67, 9);

  -- Nina - marine biologist
  INSERT INTO posts (user_id, content, post_type, media, feeling, created_at, like_count, comment_count) VALUES
  (u_nina, 'Incredible dive today! We spotted a pod of dolphins interacting with a sea turtle. Nature never ceases to amaze me. #marine #ocean', 'photo', '[{"url":"https://images.unsplash.com/photo-1544551763-46a013bb70d5?w=800","type":"image"}]', 'amazed', NOW() - interval '9 hours', 88, 14);

  INSERT INTO posts (user_id, content, post_type, created_at, like_count, comment_count) VALUES
  (u_nina, 'Did you know? The ocean produces over 50% of the world''s oxygen. Protecting our oceans is literally protecting our ability to breathe.', 'text', NOW() - interval '3 days', 124, 19);

  -- James - filmmaker
  INSERT INTO posts (user_id, content, post_type, media, created_at, like_count, comment_count) VALUES
  (u_james, 'Behind the scenes from our latest documentary shoot in the Australian outback. The landscapes here are otherworldly.', 'photo', '[{"url":"https://images.unsplash.com/photo-1506744038136-46273834b3fb?w=800","type":"image"},{"url":"https://images.unsplash.com/photo-1469474968028-56623f02e42e?w=800","type":"image"}]', NOW() - interval '10 hours', 63, 8);

  INSERT INTO posts (user_id, content, post_type, created_at, like_count, comment_count) VALUES
  (u_james, 'The best stories are the ones you discover, not the ones you plan. Always keep your camera ready.', 'text', NOW() - interval '5 days', 47, 5);

  -- ── Comments on various posts ─────────────────────────────────────────────
  -- Get recent post IDs
  FOR p_id IN SELECT id FROM posts WHERE user_id = u_sophia ORDER BY created_at DESC LIMIT 1
  LOOP
    INSERT INTO comments (user_id, post_id, content, created_at) VALUES
    (u_alex, p_id, 'Absolutely stunning! What camera did you use for this?', NOW() - interval '1 hour'),
    (u_emma, p_id, 'The colors are incredible! Barcelona sunsets hit different.', NOW() - interval '90 minutes'),
    (u_james, p_id, 'This would make an amazing film opening shot!', NOW() - interval '30 minutes');
  END LOOP;

  FOR p_id IN SELECT id FROM posts WHERE user_id = u_alex AND content LIKE '%Rust%' LIMIT 1
  LOOP
    INSERT INTO comments (user_id, post_id, content, created_at) VALUES
    (u_diego, p_id, 'Rust is the future. We are migrating our entire backend to it.', NOW() - interval '45 minutes'),
    (u_olivia, p_id, 'WASM is a game changer for web performance. Great work!', NOW() - interval '50 minutes'),
    (u_sophia, p_id, 'Even as a non-developer, I find this fascinating!', NOW() - interval '30 minutes'),
    (u_kai, p_id, 'Can you share the benchmark results? Would love to see the numbers.', NOW() - interval '20 minutes');
  END LOOP;

  FOR p_id IN SELECT id FROM posts WHERE user_id = u_luna ORDER BY created_at DESC LIMIT 1
  LOOP
    INSERT INTO comments (user_id, post_id, content, created_at) VALUES
    (u_sophia, p_id, 'The detail in this is absolutely incredible! How long did it take?', NOW() - interval '2 hours'),
    (u_olivia, p_id, 'Love the color palette. You have such a unique style!', NOW() - interval '1 hour'),
    (u_nina, p_id, 'Would you consider doing an ocean-themed piece? I would love to commission one!', NOW() - interval '90 minutes');
  END LOOP;

  FOR p_id IN SELECT id FROM posts WHERE user_id = u_emma ORDER BY created_at DESC LIMIT 1
  LOOP
    INSERT INTO comments (user_id, post_id, content, created_at) VALUES
    (u_marcus, p_id, 'This looks amazing! Is it low-carb friendly?', NOW() - interval '3 hours'),
    (u_luna, p_id, 'I need this recipe immediately!', NOW() - interval '2 hours'),
    (u_nina, p_id, 'My Italian grandmother would approve! Looks authentic.', NOW() - interval '4 hours'),
    (u_sophia, p_id, 'Made this last weekend and it was incredible. Thank you for sharing!', NOW() - interval '1 hour');
  END LOOP;

  FOR p_id IN SELECT id FROM posts WHERE user_id = u_diego AND content LIKE '%Series A%' LIMIT 1
  LOOP
    INSERT INTO comments (user_id, post_id, content, created_at) VALUES
    (u_alex, p_id, 'Congratulations Diego! Well deserved. The product is amazing.', NOW() - interval '6 hours'),
    (u_olivia, p_id, 'This is huge! Can not wait to see what you build next.', NOW() - interval '5 hours'),
    (u_james, p_id, 'Would love to document your startup journey for a short film!', NOW() - interval '4 hours');
  END LOOP;

  FOR p_id IN SELECT id FROM posts WHERE user_id = u_nina ORDER BY created_at DESC LIMIT 1
  LOOP
    INSERT INTO comments (user_id, post_id, content, created_at) VALUES
    (u_james, p_id, 'These shots are incredible! We should collaborate on an ocean documentary.', NOW() - interval '7 hours'),
    (u_sophia, p_id, 'Dolphins are my favorite! What an amazing experience.', NOW() - interval '6 hours'),
    (u_emma, p_id, 'Nature is truly the best artist. Beautiful capture!', NOW() - interval '5 hours');
  END LOOP;

  -- ── Reactions on posts ────────────────────────────────────────────────────
  INSERT INTO reactions (user_id, target_type, target_id, reaction_type)
  SELECT u_alex, 'post', p.id, 'like' FROM posts p WHERE p.user_id IN (u_sophia, u_luna, u_emma, u_diego) ORDER BY p.created_at DESC LIMIT 4
  ON CONFLICT DO NOTHING;
  INSERT INTO reactions (user_id, target_type, target_id, reaction_type)
  SELECT u_sophia, 'post', p.id, 'love' FROM posts p WHERE p.user_id IN (u_luna, u_emma, u_nina, u_james) ORDER BY p.created_at DESC LIMIT 4
  ON CONFLICT DO NOTHING;
  INSERT INTO reactions (user_id, target_type, target_id, reaction_type)
  SELECT u_luna, 'post', p.id, 'wow' FROM posts p WHERE p.user_id IN (u_sophia, u_alex, u_nina) ORDER BY p.created_at DESC LIMIT 3
  ON CONFLICT DO NOTHING;
  INSERT INTO reactions (user_id, target_type, target_id, reaction_type)
  SELECT u_marcus, 'post', p.id, 'like' FROM posts p WHERE p.user_id IN (u_alex, u_emma, u_james) ORDER BY p.created_at DESC LIMIT 3
  ON CONFLICT DO NOTHING;
  INSERT INTO reactions (user_id, target_type, target_id, reaction_type)
  SELECT u_emma, 'post', p.id, 'love' FROM posts p WHERE p.user_id IN (u_sophia, u_luna, u_marcus) ORDER BY p.created_at DESC LIMIT 3
  ON CONFLICT DO NOTHING;
  INSERT INTO reactions (user_id, target_type, target_id, reaction_type)
  SELECT u_kai, 'post', p.id, 'like' FROM posts p WHERE p.user_id IN (u_alex, u_diego, u_james) ORDER BY p.created_at DESC LIMIT 3
  ON CONFLICT DO NOTHING;
  INSERT INTO reactions (user_id, target_type, target_id, reaction_type)
  SELECT u_olivia, 'post', p.id, 'like' FROM posts p WHERE p.user_id IN (u_luna, u_alex, u_diego) ORDER BY p.created_at DESC LIMIT 3
  ON CONFLICT DO NOTHING;
  INSERT INTO reactions (user_id, target_type, target_id, reaction_type)
  SELECT u_diego, 'post', p.id, 'wow' FROM posts p WHERE p.user_id IN (u_sophia, u_nina, u_james) ORDER BY p.created_at DESC LIMIT 3
  ON CONFLICT DO NOTHING;
  INSERT INTO reactions (user_id, target_type, target_id, reaction_type)
  SELECT u_nina, 'post', p.id, 'love' FROM posts p WHERE p.user_id IN (u_sophia, u_emma, u_james) ORDER BY p.created_at DESC LIMIT 3
  ON CONFLICT DO NOTHING;
  INSERT INTO reactions (user_id, target_type, target_id, reaction_type)
  SELECT u_james, 'post', p.id, 'like' FROM posts p WHERE p.user_id IN (u_sophia, u_nina, u_diego) ORDER BY p.created_at DESC LIMIT 3
  ON CONFLICT DO NOTHING;
  -- Admin reactions
  INSERT INTO reactions (user_id, target_type, target_id, reaction_type)
  SELECT 2, 'post', p.id, (ARRAY['like','love','wow','haha'])[floor(random()*4+1)::int] FROM posts p WHERE p.user_id IN (u_sophia, u_alex, u_luna, u_emma, u_diego, u_nina) ORDER BY p.created_at DESC LIMIT 6
  ON CONFLICT DO NOTHING;

  -- ── Stories (active - expires in future) ──────────────────────────────────
  -- Sophia story
  INSERT INTO stories (user_id, created_at, expires_at) VALUES
  (u_sophia, NOW() - interval '3 hours', NOW() + interval '21 hours') RETURNING id INTO s_id;
  INSERT INTO story_media (story_id, media_type, media_url, description, duration) VALUES
  (s_id, 'image', 'https://images.unsplash.com/photo-1507525428034-b723cf961d3e?w=600', 'Golden hour at the beach', 5),
  (s_id, 'image', 'https://images.unsplash.com/photo-1476514525535-07fb3b4ae5f1?w=600', 'Adventure awaits!', 5);

  -- Alex story
  INSERT INTO stories (user_id, created_at, expires_at) VALUES
  (u_alex, NOW() - interval '5 hours', NOW() + interval '19 hours') RETURNING id INTO s_id;
  INSERT INTO story_media (story_id, media_type, media_url, description, duration) VALUES
  (s_id, 'image', 'https://images.unsplash.com/photo-1555066931-4365d14bab8c?w=600', 'Late night coding session', 5);

  -- Luna story
  INSERT INTO stories (user_id, created_at, expires_at) VALUES
  (u_luna, NOW() - interval '1 hour', NOW() + interval '23 hours') RETURNING id INTO s_id;
  INSERT INTO story_media (story_id, media_type, media_url, description, duration) VALUES
  (s_id, 'image', 'https://images.unsplash.com/photo-1460661419201-fd4cecdf8a8b?w=600', 'Work in progress', 5),
  (s_id, 'image', 'https://images.unsplash.com/photo-1513364776144-60967b0f800f?w=600', 'Almost done!', 5),
  (s_id, 'image', 'https://images.unsplash.com/photo-1547891654-e66ed7ebb968?w=600', 'Final result!', 5);

  -- Emma story
  INSERT INTO stories (user_id, created_at, expires_at) VALUES
  (u_emma, NOW() - interval '2 hours', NOW() + interval '22 hours') RETURNING id INTO s_id;
  INSERT INTO story_media (story_id, media_type, media_url, description, duration) VALUES
  (s_id, 'image', 'https://images.unsplash.com/photo-1565299624946-b28f40a0ae38?w=600', 'Homemade pizza night!', 5),
  (s_id, 'image', 'https://images.unsplash.com/photo-1484723091739-30a097e8f929?w=600', 'Dessert time', 5);

  -- Marcus story
  INSERT INTO stories (user_id, created_at, expires_at) VALUES
  (u_marcus, NOW() - interval '4 hours', NOW() + interval '20 hours') RETURNING id INTO s_id;
  INSERT INTO story_media (story_id, media_type, media_url, description, duration) VALUES
  (s_id, 'image', 'https://images.unsplash.com/photo-1534438327276-14e5300c3a48?w=600', 'Morning workout done!', 5);

  -- Diego story
  INSERT INTO stories (user_id, created_at, expires_at) VALUES
  (u_diego, NOW() - interval '6 hours', NOW() + interval '18 hours') RETURNING id INTO s_id;
  INSERT INTO story_media (story_id, media_type, media_url, description, duration) VALUES
  (s_id, 'image', 'https://images.unsplash.com/photo-1497366216548-37526070297c?w=600', 'Office vibes', 5),
  (s_id, 'image', 'https://images.unsplash.com/photo-1522071820081-009f0129c71c?w=600', 'Team meeting', 5);

  -- Nina story
  INSERT INTO stories (user_id, created_at, expires_at) VALUES
  (u_nina, NOW() - interval '7 hours', NOW() + interval '17 hours') RETURNING id INTO s_id;
  INSERT INTO story_media (story_id, media_type, media_url, description, duration) VALUES
  (s_id, 'image', 'https://images.unsplash.com/photo-1544551763-46a013bb70d5?w=600', 'Diving today!', 5),
  (s_id, 'image', 'https://images.unsplash.com/photo-1582967788606-a171c1080cb0?w=600', 'Found a sea turtle!', 5);

  -- James story
  INSERT INTO stories (user_id, created_at, expires_at) VALUES
  (u_james, NOW() - interval '8 hours', NOW() + interval '16 hours') RETURNING id INTO s_id;
  INSERT INTO story_media (story_id, media_type, media_url, description, duration) VALUES
  (s_id, 'image', 'https://images.unsplash.com/photo-1506744038136-46273834b3fb?w=600', 'On location in the outback', 5);

  -- ── Notifications for admin (id=2) ────────────────────────────────────────
  INSERT INTO notifications (recipient_id, sender_id, type, target_type, target_id, text, created_at) VALUES
  (2, u_sophia, 'Following', 'user', u_sophia, 'started following you', NOW() - interval '30 minutes'),
  (2, u_alex, 'Following', 'user', u_alex, 'started following you', NOW() - interval '1 hour'),
  (2, u_luna, 'Following', 'user', u_luna, 'started following you', NOW() - interval '2 hours'),
  (2, u_diego, 'Following', 'user', u_diego, 'started following you', NOW() - interval '3 hours'),
  (2, u_emma, 'LikedPost', 'post', 1, 'liked your post', NOW() - interval '15 minutes'),
  (2, u_marcus, 'Following', 'user', u_marcus, 'started following you', NOW() - interval '4 hours');

  -- ── Categories (page + group types) ──────────────────────────────────────
  -- NOTE: `type` is singular per schema; frontend Nearby tabs filter by slug.
  INSERT INTO categories (name_key, slug, type, active) VALUES
    ('Restaurant',           'restaurant',      'page',  true),
    ('Photography',          'photography',     'page',  true),
    ('Tech Startup',         'tech-startup',    'page',  true),
    ('Retail Shop',          'retail-shop',     'page',  true),
    ('Fitness Studio',       'fitness-studio',  'page',  true),
    ('Art Gallery',          'art-gallery',     'page',  true),
    ('Developer Community',  'developers',      'group', true),
    ('Food Lovers',          'foodies',         'group', true)
  ON CONFLICT DO NOTHING;

  DECLARE
    c_restaurant BIGINT; c_photo BIGINT; c_tech BIGINT; c_retail BIGINT; c_fitness BIGINT; c_gallery BIGINT;
    c_devs BIGINT; c_food BIGINT;
    pg_id BIGINT; gr_id BIGINT;
  BEGIN
    SELECT id INTO c_restaurant FROM categories WHERE slug='restaurant';
    SELECT id INTO c_photo      FROM categories WHERE slug='photography';
    SELECT id INTO c_tech       FROM categories WHERE slug='tech-startup';
    SELECT id INTO c_retail     FROM categories WHERE slug='retail-shop';
    SELECT id INTO c_fitness    FROM categories WHERE slug='fitness-studio';
    SELECT id INTO c_gallery    FROM categories WHERE slug='art-gallery';
    SELECT id INTO c_devs       FROM categories WHERE slug='developers';
    SELECT id INTO c_food       FROM categories WHERE slug='foodies';

    -- ── Pages (business + shops) with realistic lat/lng so /v1/pages/nearby works ─
    -- Barcelona (Sophia photography)
    INSERT INTO pages (user_id, page_name, page_title, about, category_id, address, phone, website, lat, lng, is_verified)
    VALUES (u_sophia, 'sophia_photo', 'Sophia Martinez Photography',
            'Weddings, portraits, travel. Based in Barcelona, available worldwide.',
            c_photo, 'Carrer de Mallorca 401, Barcelona', '+34 600 123 456',
            'https://sophiamartinez.photo', 41.3984, 2.1741, true)
    RETURNING id INTO pg_id;
    INSERT INTO page_likes (page_id, user_id) VALUES
      (pg_id, u_alex), (pg_id, u_luna), (pg_id, u_emma), (pg_id, 2)
    ON CONFLICT DO NOTHING;

    -- San Francisco (Alex tech)
    INSERT INTO pages (user_id, page_name, page_title, about, category_id, address, website, lat, lng, is_verified)
    VALUES (u_alex, 'techcorp_sf', 'TechCorp Engineering',
            'Rust + Wasm product engineering. We are hiring.',
            c_tech, '525 Market St, San Francisco, CA', 'https://techcorp.dev',
            37.7895, -122.3983, true)
    RETURNING id INTO pg_id;
    INSERT INTO page_likes (page_id, user_id) VALUES (pg_id, u_diego), (pg_id, 2)
    ON CONFLICT DO NOTHING;

    -- London (Emma food)
    INSERT INTO pages (user_id, page_name, page_title, about, category_id, address, phone, website, lat, lng, is_verified)
    VALUES (u_emma, 'emmas_kitchen', 'Emma''s Kitchen',
            'Seasonal menus, homemade pasta, cooking classes every Sunday.',
            c_restaurant, '23 Borough High St, London SE1', '+44 20 7946 0123',
            'https://emmaskitchen.co.uk', 51.5045, -0.0899, true)
    RETURNING id INTO pg_id;
    INSERT INTO page_likes (page_id, user_id) VALUES (pg_id, u_nina), (pg_id, u_luna), (pg_id, 2)
    ON CONFLICT DO NOTHING;

    -- Milan (Luna art gallery — shop type)
    INSERT INTO pages (user_id, page_name, page_title, about, category_id, address, lat, lng)
    VALUES (u_luna, 'luna_gallery', 'Luna Rossi Gallery',
            'Contemporary digital prints. Custom illustrations available for commission.',
            c_gallery, 'Via Tortona 18, Milan', 45.4535, 9.1601)
    RETURNING id INTO pg_id;
    INSERT INTO page_likes (page_id, user_id) VALUES (pg_id, u_sophia), (pg_id, u_olivia)
    ON CONFLICT DO NOTHING;

    -- Miami (Marcus fitness)
    INSERT INTO pages (user_id, page_name, page_title, about, category_id, address, phone, lat, lng)
    VALUES (u_marcus, 'fit_with_marcus', 'Fit With Marcus',
            'Personal training and nutrition coaching. Online + in-person in South Beach.',
            c_fitness, '1100 Ocean Dr, Miami Beach, FL', '+1 305 555 0182',
            25.7804, -80.1295)
    RETURNING id INTO pg_id;
    INSERT INTO page_likes (page_id, user_id) VALUES (pg_id, u_james), (pg_id, 2)
    ON CONFLICT DO NOTHING;

    -- Berlin (Olivia design shop)
    INSERT INTO pages (user_id, page_name, page_title, about, category_id, address, lat, lng)
    VALUES (u_olivia, 'olivia_studio', 'Olivia Studio',
            'Design resources, icon packs, UI kits. Freelance commissions welcome.',
            c_retail, 'Oranienburger Str. 27, 10117 Berlin', 52.5251, 13.3917)
    RETURNING id INTO pg_id;

    -- ── Groups ──────────────────────────────────────────────────────────────
    INSERT INTO groups (user_id, group_name, group_title, about, category_id, privacy, join_privacy, member_count)
    VALUES (u_alex, 'rust_devs', 'Rust & Web Developers',
            'Weekly discussions about Rust, Wasm, and modern web.', c_devs, 'public', 'open', 4)
    RETURNING id INTO gr_id;
    INSERT INTO group_members (group_id, user_id, role, status) VALUES
      (gr_id, u_alex, 'owner', 'active'),
      (gr_id, u_diego, 'admin', 'active'),
      (gr_id, u_kai, 'member', 'active'),
      (gr_id, 2, 'member', 'active')
    ON CONFLICT DO NOTHING;

    INSERT INTO groups (user_id, group_name, group_title, about, category_id, privacy, join_privacy, member_count)
    VALUES (u_emma, 'food_lovers', 'Food Lovers Club',
            'Share recipes, plate pics, and restaurant reviews.', c_food, 'public', 'open', 5)
    RETURNING id INTO gr_id;
    INSERT INTO group_members (group_id, user_id, role, status) VALUES
      (gr_id, u_emma, 'owner', 'active'),
      (gr_id, u_sophia, 'member', 'active'),
      (gr_id, u_luna, 'member', 'active'),
      (gr_id, u_nina, 'member', 'active'),
      (gr_id, 2, 'member', 'active')
    ON CONFLICT DO NOTHING;
  END;

  -- ── Storage providers: seed the local + R2 defaults so the admin UI has ─
  --    something to display and /v1/uploads has a working target.
  INSERT INTO storage_providers
    (name, provider_type, bucket, endpoint, region, access_key, secret_key_encrypted,
     public_url, is_active, priority)
  VALUES
    ('Local disk', 'local', 'uploads', NULL, 'local', 'local', '',
     '/uploads', true, 10),
    ('Cloudflare R2 (primary)', 'r2', 'jungle',
     'https://89ae9dc46bb0bd6c1686a28cefed9a9d.r2.cloudflarestorage.com', 'auto',
     'CHANGE_ME_ACCESS_KEY', '',
     'https://cdn.example.com', false, 100)
  ON CONFLICT (name) DO NOTHING;

  -- ── Cronjob runs: sample history so admin can visualize last runs ────────
  INSERT INTO cronjob_runs (name, status, message, duration_ms, ran_at) VALUES
    ('clean_expired_stories',   'healthy', 'Removed 12 expired stories',           342,  NOW() - interval '1 hour'),
    ('digest_weekly_memories',  'healthy', 'Dispatched 48 digest emails',         1284,  NOW() - interval '6 hours'),
    ('publish_scheduled_posts', 'healthy', 'Published 3 scheduled posts',           89,  NOW() - interval '15 minutes'),
    ('delete_old_messages',     'warning', 'retention disabled via site_config',     2,  NOW() - interval '30 minutes')
  ON CONFLICT DO NOTHING;

  -- ── Admin audit log: a representative entry ─────────────────────────────
  INSERT INTO admin_audit_log
    (admin_user_id, action, resource_type, resource_id, endpoint, status,
     changes, ip_address, user_agent, created_at)
  VALUES
    (2, 'PUT', 'settings', 'site', '/v1/admin/settings/site', 200,
     '{"site_title":"Jungle"}'::jsonb, '127.0.0.1'::inet,
     'jungle-admin/1.0', NOW() - interval '10 minutes')
  ON CONFLICT DO NOTHING;

  RAISE NOTICE 'Seed data inserted successfully!';
  RAISE NOTICE 'Users: sophia.martinez, alex.chen, luna.rossi, marcus.johnson, emma.williams, kai.nakamura, olivia.brown, diego.garcia, nina.petrova, james.taylor';
  RAISE NOTICE 'Plus 6 pages (with lat/lng), 2 groups, 2 storage providers, 4 cronjob runs, 1 audit log row.';
END $$;

COMMIT;
