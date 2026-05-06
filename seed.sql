BEGIN;

-- Idempotent demo seed: safe to run multiple times. After the first full apply,
-- a row site_config(category='seed', key='wowonder_sql_demo_v1') skips the bulk
-- insert block. To force a full re-seed, delete that row (and optionally trim
-- duplicate demo rows) before running again.

-- ── 25 fake users (password = same Argon2 hash as admin demo user) ────────
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
('james.taylor','james@example.com','$argon2id$v=19$m=19456,t=2,p=1$ZnAmvQCuH7RxtEcogWR85Q$yDHvBQMgG+w7dCjty6CisXmQsVZNpW0NzRsZIumjroA','James','Taylor','https://randomuser.me/api/portraits/men/85.jpg','https://images.unsplash.com/photo-1475924156734-496f6cac6ec1?w=1200','Adventure filmmaker and storyteller. Life begins outside your comfort zone.','male','1989-08-07','Sydney','Filmmaker at WildLens','AFTRS',true,true,true),
('riley.kim','riley@example.com','$argon2id$v=19$m=19456,t=2,p=1$ZnAmvQCuH7RxtEcogWR85Q$yDHvBQMgG+w7dCjty6CisXmQsVZNpW0NzRsZIumjroA','Riley','Kim','https://randomuser.me/api/portraits/women/12.jpg','https://images.unsplash.com/photo-1494790108377-be9c29b29330?w=1200','Product manager. Roadmaps, coffee, and user research.','female','1991-04-02','Seattle','PM at Northwind','UW',true,true,false),
('sam.oneill','sam@example.com','$argon2id$v=19$m=19456,t=2,p=1$ZnAmvQCuH7RxtEcogWR85Q$yDHvBQMgG+w7dCjty6CisXmQsVZNpW0NzRsZIumjroA','Sam','O''Neill','https://randomuser.me/api/portraits/men/41.jpg','https://images.unsplash.com/photo-1506794778202-cad84cf45f1d?w=1200','Civil engineer. Infrastructure and bridges.','male','1988-11-20','Dublin','Engineer','TCD',true,true,true),
('zoe.anderson','zoe@example.com','$argon2id$v=19$m=19456,t=2,p=1$ZnAmvQCuH7RxtEcogWR85Q$yDHvBQMgG+w7dCjty6CisXmQsVZNpW0NzRsZIumjroA','Zoe','Anderson','https://randomuser.me/api/portraits/women/90.jpg','https://images.unsplash.com/photo-1438761681033-6461ffad8d80?w=1200','Veterinarian. Cats, dogs, and weekend hiking.','female','1994-07-14','Portland','Vet clinic','OSU',true,true,false),
('morgan.lee','morgan@example.com','$argon2id$v=19$m=19456,t=2,p=1$ZnAmvQCuH7RxtEcogWR85Q$yDHvBQMgG+w7dCjty6CisXmQsVZNpW0NzRsZIumjroA','Morgan','Lee','https://randomuser.me/api/portraits/women/63.jpg','https://images.unsplash.com/photo-1534528741775-53994a69daeb?w=1200','Data scientist. NLP and fairness in ML.','nonbinary','1993-01-08','Toronto','ML lead','UofT',true,true,true),
('casey.wright','casey@example.com','$argon2id$v=19$m=19456,t=2,p=1$ZnAmvQCuH7RxtEcogWR85Q$yDHvBQMgG+w7dCjty6CisXmQsVZNpW0NzRsZIumjroA','Casey','Wright','https://randomuser.me/api/portraits/men/22.jpg','https://images.unsplash.com/photo-1500648767791-00dcc994a43e?w=1200','Teacher. High school physics and robotics club.','male','1987-05-29','Austin','Educator','UT Austin',true,true,false),
('jordan.mpls','jordan@example.com','$argon2id$v=19$m=19456,t=2,p=1$ZnAmvQCuH7RxtEcogWR85Q$yDHvBQMgG+w7dCjty6CisXmQsVZNpW0NzRsZIumjroA','Jordan','Miller','https://randomuser.me/api/portraits/women/28.jpg','https://images.unsplash.com/photo-1544005313-94ddf0286df2?w=1200','Barista and latte art competitor.','female','1999-12-01','Minneapolis','Café lead','',true,true,false),
('taylor.codes','taylor.dev@example.com','$argon2id$v=19$m=19456,t=2,p=1$ZnAmvQCuH7RxtEcogWR85Q$yDHvBQMgG+w7dCjty6CisXmQsVZNpW0NzRsZIumjroA','Taylor','Reed','https://randomuser.me/api/portraits/women/50.jpg','https://images.unsplash.com/photo-1573496359142-b8d87734a5a2?w=1200','DevRel. Docs, samples, and community.','female','1992-09-09','Denver','DevRel','',true,true,true),
('quinn.harper','quinn@example.com','$argon2id$v=19$m=19456,t=2,p=1$ZnAmvQCuH7RxtEcogWR85Q$yDHvBQMgG+w7dCjty6CisXmQsVZNpW0NzRsZIumjroA','Quinn','Harper','https://randomuser.me/api/portraits/men/55.jpg','https://images.unsplash.com/photo-1507003211169-0a1dd7228f2d?w=1200','Law student. IP and open-source licensing.','male','1998-03-22','Chicago','JD candidate','Northwestern',true,true,false),
('avery.patel','avery@example.com','$argon2id$v=19$m=19456,t=2,p=1$ZnAmvQCuH7RxtEcogWR85Q$yDHvBQMgG+w7dCjty6CisXmQsVZNpW0NzRsZIumjroA','Avery','Patel','https://randomuser.me/api/portraits/women/72.jpg','https://images.unsplash.com/photo-1487412720507-e7ab37603c6f?w=1200','Nurse. Night shifts and rock climbing.','female','1990-10-30','Houston','RN','',true,true,true),
('river.chan','river@example.com','$argon2id$v=19$m=19456,t=2,p=1$ZnAmvQCuH7RxtEcogWR85Q$yDHvBQMgG+w7dCjty6CisXmQsVZNpW0NzRsZIumjroA','River','Chan','https://randomuser.me/api/portraits/men/64.jpg','https://images.unsplash.com/photo-1472099645785-5658abf4ff4e?w=1200','Sound engineer. Live venues and podcasts.','male','1995-06-18','Vancouver','Freelance','',true,true,false),
('blake.murphy','blake@example.com','$argon2id$v=19$m=19456,t=2,p=1$ZnAmvQCuH7RxtEcogWR85Q$yDHvBQMgG+w7dCjty6CisXmQsVZNpW0NzRsZIumjroA','Blake','Murphy','https://randomuser.me/api/portraits/men/11.jpg','https://images.unsplash.com/photo-1519345182560-3f2917c472ef?w=1200','Electrician. Smart home installs.','male','1986-02-25','Phoenix','Contractor','',true,true,false),
('cameron.fox','cameron@example.com','$argon2id$v=19$m=19456,t=2,p=1$ZnAmvQCuH7RxtEcogWR85Q$yDHvBQMgG+w7dCjty6CisXmQsVZNpW0NzRsZIumjroA','Cameron','Fox','https://randomuser.me/api/portraits/women/17.jpg','https://images.unsplash.com/photo-1524504388940-b1c1722653e1?w=1200','Journalist. Local politics beat.','female','1991-08-11','Atlanta','Reporter','',true,true,true),
('drew.santos','drew@example.com','$argon2id$v=19$m=19456,t=2,p=1$ZnAmvQCuH7RxtEcogWR85Q$yDHvBQMgG+w7dCjty6CisXmQsVZNpW0NzRsZIumjroA','Drew','Santos','https://randomuser.me/api/portraits/men/36.jpg','https://images.unsplash.com/photo-1560250097-0b93528c311a?w=1200','Pilot. Cargo and flight sim streaming.','male','1984-12-05','Anchorage','Captain','',true,true,false),
('reese.morgan','reese@example.com','$argon2id$v=19$m=19456,t=2,p=1$ZnAmvQCuH7RxtEcogWR85Q$yDHvBQMgG+w7dCjty6CisXmQsVZNpW0NzRsZIumjroA','Reese','Morgan','https://randomuser.me/api/portraits/women/89.jpg','https://images.unsplash.com/photo-1580489944761-15a19d654956?w=1200','Architect. Sustainable housing.','female','1989-04-27','Copenhagen','Partner','',true,true,true),
('sky.nguyen','sky@example.com','$argon2id$v=19$m=19456,t=2,p=1$ZnAmvQCuH7RxtEcogWR85Q$yDHvBQMgG+w7dCjty6CisXmQsVZNpW0NzRsZIumjroA','Sky','Nguyen','https://randomuser.me/api/portraits/women/95.jpg','https://images.unsplash.com/photo-1531746020798-e6953c6e8e04?w=1200','Game designer. Indie narrative games.','nonbinary','1997-01-19','Singapore','Studio co-founder','',true,true,false),
('fatima.diallo','fatima@example.com','$argon2id$v=19$m=19456,t=2,p=1$ZnAmvQCuH7RxtEcogWR85Q$yDHvBQMgG+w7dCjty6CisXmQsVZNpW0NzRsZIumjroA','Fatima','Diallo','https://randomuser.me/api/portraits/women/45.jpg','https://images.unsplash.com/photo-1533106418989-88406c7cc8ca?w=1200','Fashion designer blending West African textiles with modern cuts.','female','1994-08-22','Dakar','Founder at Sankara Studio','',true,true,true),
('viktor.volkov','viktor@example.com','$argon2id$v=19$m=19456,t=2,p=1$ZnAmvQCuH7RxtEcogWR85Q$yDHvBQMgG+w7dCjty6CisXmQsVZNpW0NzRsZIumjroA','Viktor','Volkov','https://randomuser.me/api/portraits/men/44.jpg','https://images.unsplash.com/photo-1517694712202-14dd9538aa97?w=1200','Applied math professor. Topology, chaos theory, and open-source textbooks.','male','1983-03-11','Kyiv','Professor at KPI','',true,true,false),
('priya.sharma','priya@example.com','$argon2id$v=19$m=19456,t=2,p=1$ZnAmvQCuH7RxtEcogWR85Q$yDHvBQMgG+w7dCjty6CisXmQsVZNpW0NzRsZIumjroA','Priya','Sharma','https://randomuser.me/api/portraits/women/36.jpg','https://images.unsplash.com/photo-1494790108377-be9c29b29330?w=1200','Tech journalist covering startups, AI policy, and digital rights.','female','1991-12-19','Bangalore','Senior Editor at TechCrunch India','',true,true,true),
('mateo.herrera','mateo@example.com','$argon2id$v=19$m=19456,t=2,p=1$ZnAmvQCuH7RxtEcogWR85Q$yDHvBQMgG+w7dCjty6CisXmQsVZNpW0NzRsZIumjroA','Mateo','Herrera','https://randomuser.me/api/portraits/men/78.jpg','https://images.unsplash.com/photo-1556910103-1c02745aae4d?w=1200','Chef. Farm-to-table cooking, Colombian ingredients, fermentation nerd.','male','1989-06-05','Medellín','Executive Chef at Selva','',true,true,true),
('leila.abbasi','leila@example.com','$argon2id$v=19$m=19456,t=2,p=1$ZnAmvQCuH7RxtEcogWR85Q$yDHvBQMgG+w7dCjty6CisXmQsVZNpW0NzRsZIumjroA','Leila','Abbasi','https://randomuser.me/api/portraits/women/82.jpg','https://images.unsplash.com/photo-1487958449943-2429e8be8625?w=1200','Architect specializing in passive cooling and vernacular design.','female','1992-09-30','Tehran','Lead Architect at Studio Aban','',true,true,false),
('hugo.andersson','hugo@example.com','$argon2id$v=19$m=19456,t=2,p=1$ZnAmvQCuH7RxtEcogWR85Q$yDHvBQMgG+w7dCjty6CisXmQsVZNpW0NzRsZIumjroA','Hugo','Andersson','https://randomuser.me/api/portraits/men/60.jpg','https://images.unsplash.com/photo-1511512578047-dfb367046420?w=1200','Indie game dev. Solo shipping on Steam. Pixel art and procgen.','male','1996-02-14','Stockholm','Indie dev at Northern Lights Games','',true,true,false),
('yuki.tanaka','yuki@example.com','$argon2id$v=19$m=19456,t=2,p=1$ZnAmvQCuH7RxtEcogWR85Q$yDHvBQMgG+w7dCjty6CisXmQsVZNpW0NzRsZIumjroA','Yuki','Tanaka','https://randomuser.me/api/portraits/women/23.jpg','https://images.unsplash.com/photo-1438761681033-6461ffad8d80?w=1200','Small-animal vet. Rescue clinic. Adopt, do not shop.','female','1990-04-08','Osaka','Owner at Naniwa Vet Clinic','',true,true,true),
('amara.okafor','amara@example.com','$argon2id$v=19$m=19456,t=2,p=1$ZnAmvQCuH7RxtEcogWR85Q$yDHvBQMgG+w7dCjty6CisXmQsVZNpW0NzRsZIumjroA','Amara','Okafor','https://randomuser.me/api/portraits/women/55.jpg','https://images.unsplash.com/photo-1495459404783-4414ef41b43c?w=1200','Novelist. Afrofuturism, short stories, and literary translation.','female','1988-11-17','Lagos','Author','',true,true,true),
('isabella.santos','isabella@example.com','$argon2id$v=19$m=19456,t=2,p=1$ZnAmvQCuH7RxtEcogWR85Q$yDHvBQMgG+w7dCjty6CisXmQsVZNpW0NzRsZIumjroA','Isabella','Santos','https://randomuser.me/api/portraits/women/16.jpg','https://images.unsplash.com/photo-1500829243541-74b677fecc30?w=1200','Environmental scientist. Amazon restoration, carbon markets, GIS.','female','1993-07-29','São Paulo','Researcher at Amazonia Lab','USP',true,true,false),
('noah.dubois','noah@example.com','$argon2id$v=19$m=19456,t=2,p=1$ZnAmvQCuH7RxtEcogWR85Q$yDHvBQMgG+w7dCjty6CisXmQsVZNpW0NzRsZIumjroA','Noah','Dubois','https://randomuser.me/api/portraits/men/15.jpg','https://images.unsplash.com/photo-1470225620780-dba8ba36b745?w=1200','Film composer and sound designer. Orchestral + electronic hybrid scores.','male','1987-05-03','Paris','Composer at Studio Seine','Conservatoire de Paris',true,true,true)
ON CONFLICT (username) DO NOTHING;

-- Record IDs for reference
DO $$
DECLARE
  u_sophia BIGINT; u_alex BIGINT; u_luna BIGINT; u_marcus BIGINT; u_emma BIGINT;
  u_kai BIGINT; u_olivia BIGINT; u_diego BIGINT; u_nina BIGINT; u_james BIGINT;
  u_riley BIGINT; u_sam BIGINT; u_zoe BIGINT; u_morgan BIGINT; u_casey BIGINT;
  u_jordan BIGINT; u_taylor BIGINT; u_quinn BIGINT; u_avery BIGINT; u_river BIGINT;
  u_blake BIGINT; u_cameron BIGINT; u_drew BIGINT; u_reese BIGINT; u_sky BIGINT;
  u_fatima BIGINT; u_viktor BIGINT; u_priya BIGINT; u_mateo BIGINT; u_leila BIGINT;
  u_hugo BIGINT; u_yuki BIGINT; u_amara BIGINT; u_isabella BIGINT; u_noah BIGINT;
  p_id BIGINT; s_id BIGINT; c_id BIGINT;
  ev_id BIGINT; job_id BIGINT; fund_id BIGINT; prod_id BIGINT;
  sec_id BIGINT; frm_id BIGINT; th_id BIGINT;
  seed_done BOOLEAN;
BEGIN
  SELECT EXISTS (
    SELECT 1 FROM site_config WHERE category = 'seed' AND key = 'wowonder_sql_demo_v1'
  ) INTO seed_done;

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
  SELECT id INTO u_riley FROM users WHERE username='riley.kim';
  SELECT id INTO u_sam FROM users WHERE username='sam.oneill';
  SELECT id INTO u_zoe FROM users WHERE username='zoe.anderson';
  SELECT id INTO u_morgan FROM users WHERE username='morgan.lee';
  SELECT id INTO u_casey FROM users WHERE username='casey.wright';
  SELECT id INTO u_jordan FROM users WHERE username='jordan.mpls';
  SELECT id INTO u_taylor FROM users WHERE username='taylor.codes';
  SELECT id INTO u_quinn FROM users WHERE username='quinn.harper';
  SELECT id INTO u_avery FROM users WHERE username='avery.patel';
  SELECT id INTO u_river FROM users WHERE username='river.chan';
  SELECT id INTO u_blake FROM users WHERE username='blake.murphy';
  SELECT id INTO u_cameron FROM users WHERE username='cameron.fox';
  SELECT id INTO u_drew FROM users WHERE username='drew.santos';
  SELECT id INTO u_reese FROM users WHERE username='reese.morgan';
  SELECT id INTO u_sky FROM users WHERE username='sky.nguyen';
  SELECT id INTO u_fatima FROM users WHERE username='fatima.diallo';
  SELECT id INTO u_viktor FROM users WHERE username='viktor.volkov';
  SELECT id INTO u_priya FROM users WHERE username='priya.sharma';
  SELECT id INTO u_mateo FROM users WHERE username='mateo.herrera';
  SELECT id INTO u_leila FROM users WHERE username='leila.abbasi';
  SELECT id INTO u_hugo FROM users WHERE username='hugo.andersson';
  SELECT id INTO u_yuki FROM users WHERE username='yuki.tanaka';
  SELECT id INTO u_amara FROM users WHERE username='amara.okafor';
  SELECT id INTO u_isabella FROM users WHERE username='isabella.santos';
  SELECT id INTO u_noah FROM users WHERE username='noah.dubois';

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
  (u_olivia, u_diego, 'active'),(u_diego, u_olivia, 'active'),
  (u_riley, 2, 'active'),(u_sam, 2, 'active'),(u_zoe, 2, 'active'),(u_morgan, 2, 'active'),(u_casey, 2, 'active'),
  (u_jordan, 2, 'active'),(u_taylor, 2, 'active'),(u_quinn, 2, 'active'),(u_avery, 2, 'active'),(u_river, 2, 'active'),
  (u_blake, 2, 'active'),(u_cameron, 2, 'active'),(u_drew, 2, 'active'),(u_reese, 2, 'active'),(u_sky, 2, 'active'),
  (2, u_riley, 'active'),(2, u_sam, 'active'),(2, u_morgan, 'active'),(2, u_taylor, 'active'),(2, u_sky, 'active'),
  (u_riley, u_morgan, 'active'),(u_morgan, u_riley, 'active'),(u_taylor, u_alex, 'active'),(u_sky, u_luna, 'active'),
  (u_zoe, u_emma, 'active'),(u_emma, u_zoe, 'active'),(u_cameron, u_diego, 'active'),
  -- New users follow admin
  (u_fatima, 2, 'active'),(u_viktor, 2, 'active'),(u_priya, 2, 'active'),(u_mateo, 2, 'active'),
  (u_leila, 2, 'active'),(u_hugo, 2, 'active'),(u_yuki, 2, 'active'),(u_amara, 2, 'active'),
  (u_isabella, 2, 'active'),(u_noah, 2, 'active'),
  -- Admin follows back new users
  (2, u_fatima, 'active'),(2, u_priya, 'active'),(2, u_mateo, 'active'),(2, u_amara, 'active'),(2, u_noah, 'active'),
  -- Cross-follows: new users with each other and with originals
  (u_fatima, u_luna, 'active'),(u_luna, u_fatima, 'active'),
  (u_fatima, u_amara, 'active'),(u_amara, u_fatima, 'active'),
  (u_viktor, u_alex, 'active'),(u_alex, u_viktor, 'active'),
  (u_viktor, u_morgan, 'active'),(u_morgan, u_viktor, 'active'),
  (u_priya, u_diego, 'active'),(u_diego, u_priya, 'active'),
  (u_priya, u_cameron, 'active'),(u_cameron, u_priya, 'active'),
  (u_mateo, u_emma, 'active'),(u_emma, u_mateo, 'active'),
  (u_mateo, u_nina, 'active'),(u_nina, u_mateo, 'active'),
  (u_leila, u_olivia, 'active'),(u_olivia, u_leila, 'active'),
  (u_leila, u_reese, 'active'),(u_reese, u_leila, 'active'),
  (u_hugo, u_kai, 'active'),(u_kai, u_hugo, 'active'),
  (u_hugo, u_sky, 'active'),(u_sky, u_hugo, 'active'),
  (u_yuki, u_zoe, 'active'),(u_zoe, u_yuki, 'active'),
  (u_yuki, u_avery, 'active'),(u_avery, u_yuki, 'active'),
  (u_isabella, u_james, 'active'),(u_james, u_isabella, 'active'),
  (u_isabella, u_nina, 'active'),(u_nina, u_isabella, 'active'),
  (u_noah, u_sophia, 'active'),(u_sophia, u_noah, 'active'),
  (u_noah, u_river, 'active'),(u_river, u_noah, 'active')
  ON CONFLICT DO NOTHING;

  IF seed_done THEN
    RAISE NOTICE 'Demo seed already applied (site_config seed.wowonder_sql_demo_v1). Skipping bulk demo rows. Delete that site_config row to re-run the full demo insert.';
  ELSE
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

  -- ── QA: extra post types (open_to_work feed, polls, video/reel, location, colored, privacy) ──
  UPDATE users SET privacy_settings = COALESCE(privacy_settings, '{}'::jsonb) || '{"open_to_work": true}'::jsonb
  WHERE id IN (u_riley, u_morgan, u_taylor);

  INSERT INTO posts (user_id, content, post_type, privacy, created_at, like_count, comment_count) VALUES
  (u_riley, 'Open to senior PM roles in B2B SaaS (remote, US hours). Previously shipped onboarding + billing. #opentowork', 'open_to_work', 'everyone', NOW() - interval '50 minutes', 5, 1);

  INSERT INTO posts (user_id, content, post_type, privacy, media, created_at, like_count, comment_count, view_count, is_reel) VALUES
  (u_james, 'Short reel from yesterday''s shoot.', 'video', 'everyone',
   '[{"url":"https://storage.googleapis.com/gtv-videos-bucket/sample/ForBiggerBlazes.mp4","type":"video","thumbnail":"https://images.unsplash.com/photo-1506744038136-46273834b3fb?w=800"}]'::jsonb,
   NOW() - interval '40 minutes', 12, 2, 1200, true);

  INSERT INTO posts (user_id, content, post_type, privacy, media, created_at, like_count, comment_count, view_count) VALUES
  (u_james, 'Full cut: Australian outback B-roll (home feed video filter).', 'video', 'everyone',
   '[{"url":"https://storage.googleapis.com/gtv-videos-bucket/sample/ForBiggerJoyrides.mp4","type":"video"}]'::jsonb,
   NOW() - interval '28 minutes', 40, 5, 8900);

  INSERT INTO posts (user_id, content, post_type, location, lat, lng, created_at, like_count, comment_count) VALUES
  (u_sky, 'Live sketch notes from the design conference keynote.', 'text', 'Marina Bay Sands, Singapore', 1.2839, 103.8607, NOW() - interval '35 minutes', 8, 0);

  INSERT INTO posts (user_id, content, post_type, media, colored_post, created_at, like_count, comment_count) VALUES
  (u_taylor, 'Shipping docs is a feature.', 'text', '[]'::jsonb,
   '{"background":"linear-gradient(135deg, #667eea 0%, #764ba2 100%)","text_color":"#ffffff"}'::jsonb,
   NOW() - interval '30 minutes', 15, 3);

  INSERT INTO posts (user_id, content, post_type, privacy, created_at, like_count, comment_count) VALUES
  (u_quinn, 'Study group notes — friends-only visibility seed.', 'text', 'friends', NOW() - interval '20 minutes', 3, 0);

  INSERT INTO posts (user_id, content, post_type, created_at, like_count, comment_count) VALUES
  (u_morgan, 'Quick poll: which stack for the next prototype?', 'poll', NOW() - interval '25 minutes', 7, 1)
  RETURNING id INTO p_id;

  INSERT INTO polls (post_id, options) VALUES (p_id, '["Rust + Axum","Next.js + tRPC","Elixir + Phoenix"]'::jsonb);

  INSERT INTO poll_votes (poll_id, user_id, option_index)
  SELECT pl.id, u_alex, 0 FROM polls pl WHERE pl.post_id = p_id LIMIT 1;
  INSERT INTO poll_votes (poll_id, user_id, option_index)
  SELECT pl.id, u_taylor, 1 FROM polls pl WHERE pl.post_id = p_id LIMIT 1;

  INSERT INTO posts (user_id, content, post_type, scheduled_at, published_at, created_at, like_count, comment_count) VALUES
  (u_kai, 'Scheduled: new EP drops at midnight (published_at NULL until job runs).', 'text', NOW() + interval '1 day', NULL, NOW() - interval '5 minutes', 2, 0);

  -- ── Posts from new users ──────────────────────────────────────────────────
  INSERT INTO posts (user_id, content, post_type, media, feeling, created_at, like_count, comment_count) VALUES
  (u_fatima, 'Just finished a new collection inspired by Fulani beadwork. The patterns tell stories that go back centuries. #fashion #heritage', 'photo',
   '[{"url":"https://images.unsplash.com/photo-1539109136881-3be0616acf4b?w=800","type":"image"},{"url":"https://images.unsplash.com/photo-1485968579580-b6d095142e6e?w=800","type":"image"}]', 'inspired', NOW() - interval '45 minutes', 37, 6);

  INSERT INTO posts (user_id, content, post_type, created_at, like_count, comment_count) VALUES
  (u_viktor, 'New preprint: A topological approach to understanding transformer attention heads. Feedback welcome! #math #ml', 'text', NOW() - interval '2 hours', 24, 4);

  INSERT INTO posts (user_id, content, post_type, media, created_at, like_count, comment_count) VALUES
  (u_priya, 'My deep dive on AI regulation in the Global South is live. Spoke to 40+ policymakers. The TL;DR is we need equity, not just ethics. #tech #policy', 'photo',
   '[{"url":"https://images.unsplash.com/photo-1553877522-43269d4ea984?w=800","type":"image"}]', NOW() - interval '55 minutes', 56, 11);

  INSERT INTO posts (user_id, content, post_type, media, feeling, created_at, like_count, comment_count) VALUES
  (u_mateo, 'Today''s tasting menu: grilled palmito with fermented ají, plantain miso, and cocoa-nib crumble. #colombian #farmtotable', 'photo',
   '[{"url":"https://images.unsplash.com/photo-1414235077428-338989a2e8c0?w=800","type":"image"},{"url":"https://images.unsplash.com/photo-1504674900247-0877df9cc836?w=800","type":"image"}]', 'proud', NOW() - interval '1 hour', 89, 15);

  INSERT INTO posts (user_id, content, post_type, media, created_at, like_count, comment_count) VALUES
  (u_leila, 'Wind-catcher retrofit for a 1970s apartment block in Yazd. Passive cooling dropped indoor temps 12°C. #vernacular #architecture', 'photo',
   '[{"url":"https://images.unsplash.com/photo-1487958449943-2429e8be8625?w=800","type":"image"}]', NOW() - interval '3 hours', 42, 5);

  INSERT INTO posts (user_id, content, post_type, media, created_at, like_count, comment_count) VALUES
  (u_hugo, 'Wishlist page is now live on Steam! Pixel-art metroidvania with procedural biomes. 3 years of solo dev. Link in bio! #gamedev #indie', 'photo',
   '[{"url":"https://images.unsplash.com/photo-1511512578047-dfb367046420?w=800","type":"image"}]', NOW() - interval '1 hour', 118, 22);

  INSERT INTO posts (user_id, content, post_type, media, created_at, like_count, comment_count) VALUES
  (u_yuki, 'This little guy came in with a broken wing. After 6 weeks of care, he is flying again. Moments like this make it all worth it. #vetlife', 'photo',
   '[{"url":"https://images.unsplash.com/photo-1438761681033-6461ffad8d80?w=800","type":"image"}]', NOW() - interval '90 minutes', 95, 14);

  INSERT INTO posts (user_id, content, post_type, created_at, like_count, comment_count) VALUES
  (u_amara, 'First draft done! New novel set in a post-scarcity Lagos where memories are currency. Now begins the real work: revision. #amwriting #afrofuturism', 'text', NOW() - interval '4 hours', 67, 9);

  INSERT INTO posts (user_id, content, post_type, media, feeling, created_at, like_count, comment_count, location, lat, lng) VALUES
  (u_isabella, 'Fieldwork day. Soil carbon sampling across regenerated pasture vs. degraded land. The difference is visible at 2m depth.', 'photo',
   '[{"url":"https://images.unsplash.com/photo-1500829243541-74b677fecc30?w=800","type":"image"}]', 'inspired',
   NOW() - interval '5 hours', 51, 7, 'São Paulo, Brazil', -23.5505, -46.6333);

  INSERT INTO posts (user_id, content, post_type, media, created_at, like_count, comment_count) VALUES
  (u_noah, 'Scoring session today: 32-piece chamber orchestra + modular synth. The director wanted Blade Runner meets Debussy.', 'photo',
   '[{"url":"https://images.unsplash.com/photo-1470225620780-dba8ba36b745?w=800","type":"image"}]', NOW() - interval '2 hours', 43, 8);

  -- ── Page wall posts (exercising every page with varied content) ────────────
  -- sophia_photo (already has 1 post) — add a client testimonial repost
  INSERT INTO posts (user_id, page_id, content, post_type, created_at, like_count, comment_count) VALUES
  (u_sophia, (SELECT id FROM pages WHERE page_name = 'sophia_photo' LIMIT 1),
   'Client love: "Sophia captured our wedding day better than we remembered it ourselves." — Marta & Luis #wedding #barcelona', 'text', NOW() - interval '27 hours', 34, 5);

  -- techcorp_sf (already has 1 post) — add an ask-me-anything-style post
  INSERT INTO posts (user_id, page_id, content, post_type, created_at, like_count, comment_count) VALUES
  (u_alex, (SELECT id FROM pages WHERE page_name = 'techcorp_sf' LIMIT 1),
   'AMA: we migrated our core pipeline from Python → Rust. Drop your questions below and the team will answer.', 'text', NOW() - interval '13 hours', 62, 28);

  INSERT INTO posts (user_id, page_id, content, post_type, media, created_at, like_count, comment_count) VALUES
  (u_diego, (SELECT id FROM pages WHERE page_name = 'techcorp_sf' LIMIT 1),
   'Our new office in SoMa. Standing desks, nap pods, and a robot that brings you snacks.', 'photo',
   '[{"url":"https://images.unsplash.com/photo-1497366216548-37526070297c?w=800","type":"image"}]', NOW() - interval '5 days', 41, 4);

  -- emmas_kitchen (already has 1 post) — add a recipe link + a menu teaser
  INSERT INTO posts (user_id, page_id, content, post_type, created_at, like_count, comment_count) VALUES
  (u_emma, (SELECT id FROM pages WHERE page_name = 'emmas_kitchen' LIMIT 1),
   'Sunday supper menu is live! Three courses, seasonal veg, £35pp. Bookings close Friday. #london', 'text', NOW() - interval '18 hours', 28, 6);

  INSERT INTO posts (user_id, page_id, content, post_type, media, created_at, like_count, comment_count) VALUES
  (u_emma, (SELECT id FROM pages WHERE page_name = 'emmas_kitchen' LIMIT 1),
   'New video: how to caramelize onions properly (the 45-minute method is worth it).', 'photo',
   '[{"url":"https://images.unsplash.com/photo-1484723091739-30a097e8f929?w=800","type":"image"}]', NOW() - interval '3 days', 67, 11);

  -- luna_gallery — add wall posts (page exists but no wall posts yet)
  INSERT INTO posts (user_id, page_id, content, post_type, media, created_at, like_count, comment_count) VALUES
  (u_luna, (SELECT id FROM pages WHERE page_name = 'luna_gallery' LIMIT 1),
   'New exhibit opens next Friday: "Liquid Light" — 12 new digital pieces exploring water and refraction.', 'photo',
   '[{"url":"https://images.unsplash.com/photo-1579783902614-a3fb3927b6a5?w=800","type":"image"},{"url":"https://images.unsplash.com/photo-1541961017774-22349e4a1262?w=800","type":"image"}]', NOW() - interval '2 days', 84, 13);

  INSERT INTO posts (user_id, page_id, content, post_type, created_at, like_count, comment_count) VALUES
  (u_luna, (SELECT id FROM pages WHERE page_name = 'luna_gallery' LIMIT 1),
   'Behind the scenes: I have been experimenting with generative algorithms that paint in real-time from live camera input. Demo video coming soon!', 'text', NOW() - interval '5 days', 45, 9);

  -- fit_with_marcus — add workout tip posts
  INSERT INTO posts (user_id, page_id, content, post_type, media, created_at, like_count, comment_count) VALUES
  (u_marcus, (SELECT id FROM pages WHERE page_name = 'fit_with_marcus' LIMIT 1),
   'The only 5 exercises you need for a full-body home workout (no equipment). Thread:', 'photo',
   '[{"url":"https://images.unsplash.com/photo-1534438327276-14e5300c3a48?w=800","type":"image"}]', NOW() - interval '20 hours', 91, 18);

  INSERT INTO posts (user_id, page_id, content, post_type, created_at, like_count, comment_count) VALUES
  (u_marcus, (SELECT id FROM pages WHERE page_name = 'fit_with_marcus' LIMIT 1),
   'Client spotlight: Jake lost 20kg in 6 months. No crash diets, just consistency + progressive overload. If he can, you can.', 'text', NOW() - interval '4 days', 112, 22);

  -- olivia_studio — add wall posts (page has no likes or wall posts yet)
  INSERT INTO posts (user_id, page_id, content, post_type, media, created_at, like_count, comment_count) VALUES
  (u_olivia, (SELECT id FROM pages WHERE page_name = 'olivia_studio' LIMIT 1),
   'Just released a free icon pack: 200 pixel-perfect icons for dashboards. MIT license. Grab it on the site! #ux #freebies', 'photo',
   '[{"url":"https://images.unsplash.com/photo-1586717791821-3f44a563fa4c?w=800","type":"image"}]', NOW() - interval '15 hours', 76, 12);

  INSERT INTO posts (user_id, page_id, content, post_type, created_at, like_count, comment_count) VALUES
  (u_olivia, (SELECT id FROM pages WHERE page_name = 'olivia_studio' LIMIT 1),
   'Design tip: When in doubt, remove the border. White space is not wasted space — it is the breathing room your content deserves.', 'text', NOW() - interval '3 days', 134, 17);

  -- ── Group wall posts (more activity in both groups) ───────────────────────
  INSERT INTO posts (user_id, group_id, content, post_type, media, created_at, like_count, comment_count) VALUES
  (u_diego, (SELECT id FROM groups WHERE group_name = 'rust_devs' LIMIT 1),
   'We open-sourced our NATS → WebSocket bridge. Production-tested at 50k concurrent connections. #rustlang', 'photo',
   '[{"url":"https://images.unsplash.com/photo-1555066931-4365d14bab8c?w=800","type":"image"}]', NOW() - interval '6 hours', 47, 9);

  INSERT INTO posts (user_id, group_id, content, post_type, created_at, like_count, comment_count) VALUES
  (u_kai, (SELECT id FROM groups WHERE group_name = 'rust_devs' LIMIT 1),
   'Any recommendations for real-time audio processing crates? Building a collaborative DAW prototype.', 'text', NOW() - interval '12 hours', 18, 12);

  INSERT INTO posts (user_id, group_id, content, post_type, media, created_at, like_count, comment_count) VALUES
  (u_sophia, (SELECT id FROM groups WHERE group_name = 'food_lovers' LIMIT 1),
   'Best paella I had this year — shrimp, squid, and foraged mushrooms from the Catalan hills.', 'photo',
   '[{"url":"https://images.unsplash.com/photo-1473093295043-cdd812d0e601?w=800","type":"image"}]', NOW() - interval '8 hours', 53, 11);

  INSERT INTO posts (user_id, group_id, content, post_type, created_at, like_count, comment_count) VALUES
  (u_mateo, (SELECT id FROM groups WHERE group_name = 'food_lovers' LIMIT 1),
   'Fermentation station: 6-month aged gochujang, tepache, and a wild yeast sourdough starter. Ask me anything about ferments.', 'text', NOW() - interval '14 hours', 39, 17);

  -- ── Page invites (users inviting friends to pages) ─────────────────────────
  INSERT INTO page_invites (page_id, inviter_id, invited_id)
  SELECT pg.id, u_sophia, u_fatima FROM pages pg WHERE pg.page_name = 'sophia_photo' LIMIT 1
  ON CONFLICT DO NOTHING;
  INSERT INTO page_invites (page_id, inviter_id, invited_id)
  SELECT pg.id, u_emma, u_mateo FROM pages pg WHERE pg.page_name = 'emmas_kitchen' LIMIT 1
  ON CONFLICT DO NOTHING;
  INSERT INTO page_invites (page_id, inviter_id, invited_id)
  SELECT pg.id, u_alex, u_viktor FROM pages pg WHERE pg.page_name = 'techcorp_sf' LIMIT 1
  ON CONFLICT DO NOTHING;
  INSERT INTO page_invites (page_id, inviter_id, invited_id)
  SELECT pg.id, u_luna, u_leila FROM pages pg WHERE pg.page_name = 'luna_gallery' LIMIT 1
  ON CONFLICT DO NOTHING;

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

  -- Comments on new users' posts
  FOR p_id IN SELECT id FROM posts WHERE user_id = u_fatima ORDER BY created_at DESC LIMIT 1
  LOOP
    INSERT INTO comments (user_id, post_id, content, created_at) VALUES
    (u_luna, p_id, 'The beadwork detail is stunning! Do you also do custom commissions?', NOW() - interval '35 minutes'),
    (u_amara, p_id, 'As a fellow West African designer — this is breathtaking. The patterns remind me of my grandmother''s collection.', NOW() - interval '20 minutes');
  END LOOP;

  FOR p_id IN SELECT id FROM posts WHERE user_id = u_mateo ORDER BY created_at DESC LIMIT 1
  LOOP
    INSERT INTO comments (user_id, post_id, content, created_at) VALUES
    (u_emma, p_id, 'Palmito + fermented ají is a genius combo. What is your fermentation setup?', NOW() - interval '45 minutes'),
    (u_nina, p_id, 'I would fly to Medellín just for this tasting menu. Absolutely gorgeous plating.', NOW() - interval '30 minutes'),
    (u_priya, p_id, 'Farm-to-table done right is a form of activism. Respect!', NOW() - interval '15 minutes');
  END LOOP;

  FOR p_id IN SELECT id FROM posts WHERE user_id = u_hugo ORDER BY created_at DESC LIMIT 1
  LOOP
    INSERT INTO comments (user_id, post_id, content, created_at) VALUES
    (u_kai, p_id, 'Wishlisted! Pixel-art + procgen is my favorite combo. Any chance of a soundtrack preview?', NOW() - interval '40 minutes'),
    (u_sky, p_id, '3 years of solo dev is incredible. Fellow indie — sending support from Singapore!', NOW() - interval '25 minutes');
  END LOOP;

  FOR p_id IN SELECT id FROM posts WHERE user_id = u_yuki ORDER BY created_at DESC LIMIT 1
  LOOP
    INSERT INTO comments (user_id, post_id, content, created_at) VALUES
    (u_zoe, p_id, 'You are a hero! The vet clinic I work at sees so many wing injuries this time of year.', NOW() - interval '70 minutes'),
    (u_avery, p_id, 'Rescue work is the hardest and most rewarding. Thank you for what you do.', NOW() - interval '50 minutes');
  END LOOP;

  FOR p_id IN SELECT id FROM posts WHERE user_id = u_noah ORDER BY created_at DESC LIMIT 1
  LOOP
    INSERT INTO comments (user_id, post_id, content, created_at) VALUES
    (u_james, p_id, '32-piece orchestra + modular synth? You absolute mad scientist. I need to hear this!', NOW() - interval '1 hour'),
    (u_river, p_id, 'Debussy + Blade Runner is the aesthetic we all need. Session photos please!', NOW() - interval '40 minutes');
  END LOOP;

  -- Comments on page wall posts
  FOR p_id IN SELECT id FROM posts WHERE page_id = (SELECT id FROM pages WHERE page_name = 'luna_gallery' LIMIT 1) ORDER BY created_at DESC LIMIT 1
  LOOP
    INSERT INTO comments (user_id, post_id, content, created_at) VALUES
    (u_sophia, p_id, 'Cannot wait for the opening! Will it be streamed for those of us not in Milan?', NOW() - interval '1 day'),
    (u_olivia, p_id, 'Liquid Light is such an evocative theme. Your use of color has been incredible lately!', NOW() - interval '22 hours'),
    (u_fatima, p_id, 'I would love to collaborate on a textile + digital art crossover piece someday!', NOW() - interval '18 hours');
  END LOOP;

  FOR p_id IN SELECT id FROM posts WHERE page_id = (SELECT id FROM pages WHERE page_name = 'fit_with_marcus' LIMIT 1) ORDER BY created_at DESC LIMIT 1
  LOOP
    INSERT INTO comments (user_id, post_id, content, created_at) VALUES
    (u_james, p_id, 'Been doing these for 2 months and already feel stronger. Thanks for sharing genuinely useful content!', NOW() - interval '16 hours'),
    (u_mateo, p_id, 'As a chef who stands 14 hours a day, a strong core is essential. Great tips.', NOW() - interval '10 hours');
  END LOOP;

  FOR p_id IN SELECT id FROM posts WHERE page_id = (SELECT id FROM pages WHERE page_name = 'olivia_studio' LIMIT 1) ORDER BY created_at DESC LIMIT 1
  LOOP
    INSERT INTO comments (user_id, post_id, content, created_at) VALUES
    (u_leila, p_id, 'Using these icons in my architecture portfolio presentation. Perfectly crisp!', NOW() - interval '10 hours'),
    (u_priya, p_id, 'MIT license is chef''s kiss. Added to our design system resources list.', NOW() - interval '6 hours'),
    (u_taylor, p_id, 'Any plans for a variable-font version? Would complement the icon pack beautifully.', NOW() - interval '3 hours');
  END LOOP;

  -- ── Conversations + messages (DMs between users) ──────────────────────────
  -- Conv 1: Alex + Sophia (dev + photographer chatting about a collab)
  INSERT INTO conversations (type, last_message_at) VALUES ('direct', NOW() - interval '15 minutes')
  RETURNING id INTO c_id;
  INSERT INTO conversation_members (conversation_id, user_id) VALUES (c_id, u_alex), (c_id, u_sophia);
  INSERT INTO messages (conversation_id, sender_id, content, message_type, created_at) VALUES
  (c_id, u_alex, 'Hey Sophia! Love your sunset shots. Would you be open to doing some photography for our new office?', 'text', NOW() - interval '2 hours'),
  (c_id, u_sophia, 'Thanks Alex! I would love to. What kind of shots are you looking for?', 'text', NOW() - interval '90 minutes'),
  (c_id, u_alex, 'Team portraits + some architectural shots of the space. Budget is around $2k.', 'text', NOW() - interval '1 hour'),
  (c_id, u_sophia, 'Sounds great. Let me check my calendar and get back to you tomorrow!', 'text', NOW() - interval '30 minutes'),
  (c_id, u_alex, 'Perfect! I will send over the brief in the morning.', 'text', NOW() - interval '15 minutes');

  -- Conv 2: Emma + Mateo (chefs exchanging recipes)
  INSERT INTO conversations (type, last_message_at) VALUES ('direct', NOW() - interval '45 minutes')
  RETURNING id INTO c_id;
  INSERT INTO conversation_members (conversation_id, user_id) VALUES (c_id, u_emma), (c_id, u_mateo);
  INSERT INTO messages (conversation_id, sender_id, content, message_type, created_at) VALUES
  (c_id, u_emma, 'Mateo! That palmito dish looked incredible. Would you share your fermentation method?', 'text', NOW() - interval '3 hours'),
  (c_id, u_mateo, 'Of course! The trick is using wild-harvested ají from the Chocó region. I can ship you some starter culture if you want!', 'text', NOW() - interval '2 hours'),
  (c_id, u_emma, 'That would be amazing! In return I will send you my sourdough starter — it is 8 years old.', 'text', NOW() - interval '1 hour'),
  (c_id, u_mateo, '8 years! Deal. Fermentation friends for life.', 'text', NOW() - interval '45 minutes');

  -- Conv 3: Group chat — Rust Devs
  INSERT INTO conversations (type, name, last_message_at) VALUES ('group', 'Rust Devs Core', NOW() - interval '25 minutes')
  RETURNING id INTO c_id;
  INSERT INTO conversation_members (conversation_id, user_id, role) VALUES
  (c_id, u_alex, 'owner'), (c_id, u_diego, 'admin'), (c_id, u_kai, 'member'), (c_id, u_viktor, 'member'), (c_id, u_priya, 'member');
  INSERT INTO messages (conversation_id, sender_id, content, message_type, created_at) VALUES
  (c_id, u_alex, 'Welcome to the Rust core group! Viktor and Priya just joined — great to have you both.', 'text', NOW() - interval '5 hours'),
  (c_id, u_viktor, 'Thanks Alex! Excited to connect with more systems folks.', 'text', NOW() - interval '4 hours'),
  (c_id, u_diego, 'Quick poll: who is going to RustConf this year? We should do a group meetup.', 'text', NOW() - interval '3 hours'),
  (c_id, u_kai, 'I will be there! Presenting on real-time audio processing with Rust.', 'text', NOW() - interval '2 hours'),
  (c_id, u_priya, 'Attending as press. Would love to interview the team for TechCrunch!', 'text', NOW() - interval '1 hour'),
  (c_id, u_alex, 'Incredible! Let us coordinate a Birds of a Feather session.', 'text', NOW() - interval '25 minutes');

  -- Conv 4: Luna + Fatima + Amara (creative group chat)
  INSERT INTO conversations (type, name, last_message_at) VALUES ('group', 'Creative Crossroads', NOW() - interval '50 minutes')
  RETURNING id INTO c_id;
  INSERT INTO conversation_members (conversation_id, user_id, role) VALUES
  (c_id, u_luna, 'owner'), (c_id, u_fatima, 'member'), (c_id, u_amara, 'member'), (c_id, u_noah, 'member');
  INSERT INTO messages (conversation_id, sender_id, content, message_type, created_at) VALUES
  (c_id, u_luna, 'OK hear me out: a collaborative project blending digital art, fashion, and sound design. Amara can write the story, Fatima does the textiles, Noah scores it.', 'text', NOW() - interval '4 hours'),
  (c_id, u_fatima, 'I am SO in. I have been wanting to explore digital pattern projection on fabric.', 'text', NOW() - interval '3 hours'),
  (c_id, u_amara, 'I already have a narrative fragment that would be perfect for this. Post-scarcity market where clothing stores memories.', 'text', NOW() - interval '2 hours'),
  (c_id, u_noah, 'And I have been experimenting with conductive thread + capacitive sensing for interactive garments. The sound could respond to touch!', 'text', NOW() - interval '50 minutes');

  -- ── Saved posts ───────────────────────────────────────────────────────────
  INSERT INTO saved_posts (user_id, post_id)
  SELECT u_emma, p.id FROM posts p WHERE p.user_id = u_mateo ORDER BY p.created_at DESC LIMIT 1
  ON CONFLICT DO NOTHING;
  INSERT INTO saved_posts (user_id, post_id)
  SELECT u_alex, p.id FROM posts p WHERE p.user_id = u_hugo ORDER BY p.created_at DESC LIMIT 1
  ON CONFLICT DO NOTHING;
  INSERT INTO saved_posts (user_id, post_id)
  SELECT u_luna, p.id FROM posts p WHERE p.user_id = u_fatima ORDER BY p.created_at DESC LIMIT 1
  ON CONFLICT DO NOTHING;
  INSERT INTO saved_posts (user_id, post_id)
  SELECT u_nina, p.id FROM posts p WHERE p.user_id = u_mateo ORDER BY p.created_at DESC LIMIT 1
  ON CONFLICT DO NOTHING;
  INSERT INTO saved_posts (user_id, post_id)
  SELECT u_amara, p.id FROM posts p WHERE p.user_id = u_luna ORDER BY p.created_at DESC LIMIT 1
  ON CONFLICT DO NOTHING;
  INSERT INTO saved_posts (user_id, post_id)
  SELECT u_noah, p.id FROM posts p WHERE p.user_id = u_james ORDER BY p.created_at DESC LIMIT 1
  ON CONFLICT DO NOTHING;

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

  -- New users reactions
  INSERT INTO reactions (user_id, target_type, target_id, reaction_type)
  SELECT u_fatima, 'post', p.id, 'love' FROM posts p WHERE p.user_id IN (u_luna, u_amara, u_sophia) ORDER BY p.created_at DESC LIMIT 3
  ON CONFLICT DO NOTHING;
  INSERT INTO reactions (user_id, target_type, target_id, reaction_type)
  SELECT u_viktor, 'post', p.id, 'like' FROM posts p WHERE p.user_id IN (u_alex, u_morgan, u_hugo) ORDER BY p.created_at DESC LIMIT 3
  ON CONFLICT DO NOTHING;
  INSERT INTO reactions (user_id, target_type, target_id, reaction_type)
  SELECT u_priya, 'post', p.id, 'wow' FROM posts p WHERE p.user_id IN (u_diego, u_fatima, u_isabella) ORDER BY p.created_at DESC LIMIT 3
  ON CONFLICT DO NOTHING;
  INSERT INTO reactions (user_id, target_type, target_id, reaction_type)
  SELECT u_mateo, 'post', p.id, 'love' FROM posts p WHERE p.user_id IN (u_emma, u_nina, u_fatima) ORDER BY p.created_at DESC LIMIT 3
  ON CONFLICT DO NOTHING;
  INSERT INTO reactions (user_id, target_type, target_id, reaction_type)
  SELECT u_noah, 'post', p.id, 'like' FROM posts p WHERE p.user_id IN (u_james, u_river, u_kai) ORDER BY p.created_at DESC LIMIT 3
  ON CONFLICT DO NOTHING;
  INSERT INTO reactions (user_id, target_type, target_id, reaction_type)
  SELECT u_isabella, 'post', p.id, 'love' FROM posts p WHERE p.user_id IN (u_nina, u_james, u_yuki) ORDER BY p.created_at DESC LIMIT 3
  ON CONFLICT DO NOTHING;

  -- ── Stories for new users ──────────────────────────────────────────────────
  -- Fatima story
  INSERT INTO stories (user_id, created_at, expires_at) VALUES
  (u_fatima, NOW() - interval '2 hours', NOW() + interval '22 hours') RETURNING id INTO s_id;
  INSERT INTO story_media (story_id, media_type, media_url, description, duration) VALUES
  (s_id, 'image', 'https://images.unsplash.com/photo-1539109136881-3be0616acf4b?w=600', 'New fabric samples arrived!', 5),
  (s_id, 'image', 'https://images.unsplash.com/photo-1485968579580-b6d095142e6e?w=600', 'Sketching the next collection', 5);

  -- Mateo story
  INSERT INTO stories (user_id, created_at, expires_at) VALUES
  (u_mateo, NOW() - interval '3 hours', NOW() + interval '21 hours') RETURNING id INTO s_id;
  INSERT INTO story_media (story_id, media_type, media_url, description, duration) VALUES
  (s_id, 'image', 'https://images.unsplash.com/photo-1414235077428-338989a2e8c0?w=600', 'Tonight''s special: tuna tostada', 5),
  (s_id, 'image', 'https://images.unsplash.com/photo-1504674900247-0877df9cc836?w=600', 'Kitchen garden harvest', 5);

  -- Hugo story
  INSERT INTO stories (user_id, created_at, expires_at) VALUES
  (u_hugo, NOW() - interval '1 hour', NOW() + interval '23 hours') RETURNING id INTO s_id;
  INSERT INTO story_media (story_id, media_type, media_url, description, duration) VALUES
  (s_id, 'image', 'https://images.unsplash.com/photo-1511512578047-dfb367046420?w=600', 'New boss battle animation', 5);

  -- Amara story
  INSERT INTO stories (user_id, created_at, expires_at) VALUES
  (u_amara, NOW() - interval '4 hours', NOW() + interval '20 hours') RETURNING id INTO s_id;
  INSERT INTO story_media (story_id, media_type, media_url, description, duration) VALUES
  (s_id, 'image', 'https://images.unsplash.com/photo-1495459404783-4414ef41b43c?w=600', 'Writing by the lagoon. Chapter 4.', 5);

  -- Noah story
  INSERT INTO stories (user_id, created_at, expires_at) VALUES
  (u_noah, NOW() - interval '6 hours', NOW() + interval '18 hours') RETURNING id INTO s_id;
  INSERT INTO story_media (story_id, media_type, media_url, description, duration) VALUES
  (s_id, 'image', 'https://images.unsplash.com/photo-1470225620780-dba8ba36b745?w=600', 'Scoring session in progress', 5);

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
  (2, u_marcus, 'Following', 'user', u_marcus, 'started following you', NOW() - interval '4 hours'),
  (2, u_fatima, 'Following', 'user', u_fatima, 'started following you', NOW() - interval '22 minutes'),
  (2, u_priya, 'Following', 'user', u_priya, 'started following you', NOW() - interval '45 minutes'),
  (2, u_mateo, 'Following', 'user', u_mateo, 'started following you', NOW() - interval '50 minutes'),
  (2, u_amara, 'Following', 'user', u_amara, 'started following you', NOW() - interval '35 minutes'),
  (2, u_noah, 'Following', 'user', u_noah, 'started following you', NOW() - interval '40 minutes');

  -- Notifications for new users (so their notification bell is populated too)
  INSERT INTO notifications (recipient_id, sender_id, type, target_type, target_id, text, created_at) VALUES
  (u_fatima, u_luna, 'LikedPost', 'post', -1, 'reacted to your post', NOW() - interval '35 minutes'),
  (u_mateo, u_emma, 'LikedPost', 'post', -1, 'reacted to your post', NOW() - interval '40 minutes'),
  (u_hugo, u_kai, 'LikedPost', 'post', -1, 'reacted to your post', NOW() - interval '30 minutes'),
  (u_yuki, u_zoe, 'Following', 'user', u_zoe, 'started following you', NOW() - interval '55 minutes'),
  (u_amara, u_fatima, 'Following', 'user', u_fatima, 'started following you', NOW() - interval '60 minutes'),
  (u_noah, u_sophia, 'Following', 'user', u_sophia, 'started following you', NOW() - interval '40 minutes');

  -- ── Categories (page + group types) ──────────────────────────────────────
  -- NOTE: `type` is singular per schema; frontend Nearby tabs filter by slug.
  INSERT INTO categories (name_key, slug, type, active, sort_order)
  SELECT v.name_key, v.slug, v.type, v.active, v.sort_order
  FROM (VALUES
    ('Restaurant',           'restaurant',       'page',  true, 10),
    ('Photography',          'photography',      'page',  true, 20),
    ('Tech Startup',         'tech-startup',     'page',  true, 30),
    ('Retail Shop',          'retail-shop',      'page',  true, 40),
    ('Fitness Studio',       'fitness-studio',   'page',  true, 50),
    ('Art Gallery',          'art-gallery',      'page',  true, 60),
    ('Coffee Shop',          'coffee-shop',      'page',  true, 70),
    ('Music Venue',          'music-venue',      'page',  true, 80),
    ('Nonprofit',            'nonprofit',        'page',  true, 90),
    ('Travel Agency',        'travel-agency',    'page',  true, 100),
    ('Developer Community',  'developers',       'group', true, 10),
    ('Food Lovers',          'foodies',          'group', true, 20),
    ('Travel Club',          'travelers',        'group', true, 30),
    ('Music Producers',      'music-producers',  'group', true, 40),
    ('Open Source',          'open-source',      'group', true, 50),
    ('Technology',           'blog-tech',        'blog',  true, 10),
    ('Lifestyle',            'blog-lifestyle',   'blog',  true, 20),
    ('Career',               'blog-career',      'blog',  true, 30),
    ('Software Engineering', 'job-software',     'job',   true, 10),
    ('Design & Creative',    'job-design',       'job',   true, 20),
    ('Healthcare',           'job-healthcare',   'job',   true, 30),
    ('Electronics',          'product-electronics','product', true, 10),
    ('Handmade',             'product-handmade', 'product', true, 20),
    ('Sports & Outdoors',    'product-sports',   'product', true, 30)
  ) AS v(name_key, slug, type, active, sort_order)
  WHERE NOT EXISTS (SELECT 1 FROM categories c WHERE c.slug = v.slug AND c.type = v.type);

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
    ON CONFLICT (page_name) DO NOTHING;
    SELECT id INTO pg_id FROM pages WHERE page_name = 'sophia_photo' LIMIT 1;
    INSERT INTO page_likes (page_id, user_id) VALUES
      (pg_id, u_alex), (pg_id, u_luna), (pg_id, u_emma), (pg_id, 2)
    ON CONFLICT DO NOTHING;

    -- San Francisco (Alex tech)
    INSERT INTO pages (user_id, page_name, page_title, about, category_id, address, website, lat, lng, is_verified)
    VALUES (u_alex, 'techcorp_sf', 'TechCorp Engineering',
            'Rust + Wasm product engineering. We are hiring.',
            c_tech, '525 Market St, San Francisco, CA', 'https://techcorp.dev',
            37.7895, -122.3983, true)
    ON CONFLICT (page_name) DO NOTHING;
    SELECT id INTO pg_id FROM pages WHERE page_name = 'techcorp_sf' LIMIT 1;
    INSERT INTO page_likes (page_id, user_id) VALUES (pg_id, u_diego), (pg_id, 2)
    ON CONFLICT DO NOTHING;

    -- London (Emma food)
    INSERT INTO pages (user_id, page_name, page_title, about, category_id, address, phone, website, lat, lng, is_verified)
    VALUES (u_emma, 'emmas_kitchen', 'Emma''s Kitchen',
            'Seasonal menus, homemade pasta, cooking classes every Sunday.',
            c_restaurant, '23 Borough High St, London SE1', '+44 20 7946 0123',
            'https://emmaskitchen.co.uk', 51.5045, -0.0899, true)
    ON CONFLICT (page_name) DO NOTHING;
    SELECT id INTO pg_id FROM pages WHERE page_name = 'emmas_kitchen' LIMIT 1;
    INSERT INTO page_likes (page_id, user_id) VALUES (pg_id, u_nina), (pg_id, u_luna), (pg_id, 2)
    ON CONFLICT DO NOTHING;

    -- Milan (Luna art gallery — shop type)
    INSERT INTO pages (user_id, page_name, page_title, about, category_id, address, lat, lng)
    VALUES (u_luna, 'luna_gallery', 'Luna Rossi Gallery',
            'Contemporary digital prints. Custom illustrations available for commission.',
            c_gallery, 'Via Tortona 18, Milan', 45.4535, 9.1601)
    ON CONFLICT (page_name) DO NOTHING;
    SELECT id INTO pg_id FROM pages WHERE page_name = 'luna_gallery' LIMIT 1;
    INSERT INTO page_likes (page_id, user_id) VALUES (pg_id, u_sophia), (pg_id, u_olivia)
    ON CONFLICT DO NOTHING;

    -- Miami (Marcus fitness)
    INSERT INTO pages (user_id, page_name, page_title, about, category_id, address, phone, lat, lng)
    VALUES (u_marcus, 'fit_with_marcus', 'Fit With Marcus',
            'Personal training and nutrition coaching. Online + in-person in South Beach.',
            c_fitness, '1100 Ocean Dr, Miami Beach, FL', '+1 305 555 0182',
            25.7804, -80.1295)
    ON CONFLICT (page_name) DO NOTHING;
    SELECT id INTO pg_id FROM pages WHERE page_name = 'fit_with_marcus' LIMIT 1;
    INSERT INTO page_likes (page_id, user_id) VALUES (pg_id, u_james), (pg_id, 2)
    ON CONFLICT DO NOTHING;

    -- Berlin (Olivia design shop)
    INSERT INTO pages (user_id, page_name, page_title, about, category_id, address, lat, lng)
    VALUES (u_olivia, 'olivia_studio', 'Olivia Studio',
            'Design resources, icon packs, UI kits. Freelance commissions welcome.',
            c_retail, 'Oranienburger Str. 27, 10117 Berlin', 52.5251, 13.3917)
    ON CONFLICT (page_name) DO NOTHING;
    SELECT id INTO pg_id FROM pages WHERE page_name = 'olivia_studio' LIMIT 1;
    INSERT INTO page_likes (page_id, user_id) VALUES
      (pg_id, u_leila), (pg_id, u_priya), (pg_id, u_taylor), (pg_id, 2)
    ON CONFLICT DO NOTHING;

    -- ── Groups ──────────────────────────────────────────────────────────────
    INSERT INTO groups (user_id, group_name, group_title, about, category_id, privacy, join_privacy, member_count)
    VALUES (u_alex, 'rust_devs', 'Rust & Web Developers',
            'Weekly discussions about Rust, Wasm, and modern web.', c_devs, 'public', 'open', 4)
    ON CONFLICT (group_name) DO NOTHING;
    SELECT id INTO gr_id FROM groups WHERE group_name = 'rust_devs' LIMIT 1;
    INSERT INTO group_members (group_id, user_id, role, status) VALUES
      (gr_id, u_alex, 'owner', 'active'),
      (gr_id, u_diego, 'admin', 'active'),
      (gr_id, u_kai, 'member', 'active'),
      (gr_id, 2, 'member', 'active')
    ON CONFLICT DO NOTHING;

    INSERT INTO groups (user_id, group_name, group_title, about, category_id, privacy, join_privacy, member_count)
    VALUES (u_emma, 'food_lovers', 'Food Lovers Club',
            'Share recipes, plate pics, and restaurant reviews.', c_food, 'public', 'open', 5)
    ON CONFLICT (group_name) DO NOTHING;
    SELECT id INTO gr_id FROM groups WHERE group_name = 'food_lovers' LIMIT 1;
    INSERT INTO group_members (group_id, user_id, role, status) VALUES
      (gr_id, u_emma, 'owner', 'active'),
      (gr_id, u_sophia, 'member', 'active'),
      (gr_id, u_luna, 'member', 'active'),
      (gr_id, u_nina, 'member', 'active'),
      (gr_id, 2, 'member', 'active')
    ON CONFLICT DO NOTHING;
  END;

  -- ── Events + wall post + RSVPs ───────────────────────────────────────────
  INSERT INTO events (creator_id, name, description, location, start_at, end_at) VALUES
  (u_emma, 'Community Pasta Night', 'Bring a dish — gluten-free table available.', 'Emma''s Kitchen, London',
   NOW() + interval '2 days', NOW() + interval '2 days' + interval '3 hours')
  RETURNING id INTO ev_id;

  INSERT INTO posts (user_id, event_id, content, post_type, created_at, like_count, comment_count) VALUES
  (u_emma, ev_id, 'Who is coming to pasta night? RSVP below.', 'event', NOW() - interval '8 hours', 20, 6);

  INSERT INTO event_responses (event_id, user_id, response) VALUES
  (ev_id, 2, 'going'),
  (ev_id, u_sophia, 'interested'),
  (ev_id, u_luna, 'going'),
  (ev_id, u_nina, 'not_going')
  ON CONFLICT DO NOTHING;

  -- ── Page + group wall posts (exercise page_id / group_id feeds) ─────────
  INSERT INTO posts (user_id, page_id, content, post_type, created_at, like_count, comment_count) VALUES
  (u_sophia, (SELECT id FROM pages WHERE page_name = 'sophia_photo' LIMIT 1),
   'Weekend mini sessions: two slots left. DM to book!', 'text', NOW() - interval '11 hours', 6, 2);

  INSERT INTO posts (user_id, group_id, content, post_type, created_at, like_count, comment_count) VALUES
  (u_alex, (SELECT id FROM groups WHERE group_name = 'rust_devs' LIMIT 1),
   'Benchmark notes: serde_json vs simd-json on a 10MB payload (cold cache).', 'text', NOW() - interval '9 hours', 14, 4);

  -- ── Blogs + comment ──────────────────────────────────────────────────────
  INSERT INTO blogs (user_id, title, content, description, category_id, tags, view_count, share_count) VALUES
  (u_cameron, 'City council recap — Q1 zoning changes',
   'Agenda items included mixed-use permits and transit-oriented development. Here is what mattered for residents...',
   'Notes from public meetings; not legal advice.',
   (SELECT id FROM categories WHERE slug = 'blog-tech' LIMIT 1),
   ARRAY['politics','local','zoning'], 142, 3);

  INSERT INTO blog_comments (blog_id, user_id, content)
  SELECT b.id, 2, 'Clear summary — thanks for posting.'
  FROM blogs b WHERE b.title = 'City council recap — Q1 zoning changes' LIMIT 1;
  INSERT INTO blog_comments (blog_id, user_id, content)
  SELECT b.id, u_quinn, 'The transit-oriented development piece is huge. Any word on timeline?'
  FROM blogs b WHERE b.title = 'City council recap — Q1 zoning changes' LIMIT 1;
  INSERT INTO blog_comments (blog_id, user_id, content)
  SELECT b.id, u_priya, 'Great reporting. Local politics coverage like this is so important.'
  FROM blogs b WHERE b.title = 'City council recap — Q1 zoning changes' LIMIT 1;

  INSERT INTO blogs (user_id, title, content, description, category_id, tags, view_count) VALUES
  (u_river, 'Mixing live podcasts: gain staging cheatsheet',
   'Start conservative on preamp gain; compress in stages...', 'Audio engineering',
   (SELECT id FROM categories WHERE slug = 'blog-lifestyle' LIMIT 1),
   ARRAY['audio','production'], 58);

  INSERT INTO blog_comments (blog_id, user_id, content)
  SELECT b.id, u_noah, 'Great writeup. I would add: always high-pass at 80Hz for spoken word before the compressor hits.'
  FROM blogs b WHERE b.title = 'Mixing live podcasts: gain staging cheatsheet' LIMIT 1;
  INSERT INTO blog_comments (blog_id, user_id, content)
  SELECT b.id, u_kai, 'Been using your gain structure method on my last 3 live sets. Cleanest mixes I have had.'
  FROM blogs b WHERE b.title = 'Mixing live podcasts: gain staging cheatsheet' LIMIT 1;

  -- New blog posts
  INSERT INTO blogs (user_id, title, content, description, category_id, tags, view_count, share_count) VALUES
  (u_fatima, 'The case for slow fashion in West Africa',
   'Fast fashion is the second-largest polluter globally. In Dakar, a growing movement of designers is reclaiming traditional techniques — hand-dyeing, hand-weaving, and zero-waste cutting — to build a fashion ecosystem that honors both people and planet...',
   'Why traditional techniques are the future of sustainable fashion.',
   (SELECT id FROM categories WHERE slug = 'blog-lifestyle' LIMIT 1),
   ARRAY['fashion','sustainability','africa'], 210, 12);

  INSERT INTO blogs (user_id, title, content, description, category_id, tags, view_count) VALUES
  (u_viktor, 'Why topology matters for machine learning (with code)',
   'Persistent homology and the mapper algorithm reveal structure in high-dimensional data that linear methods miss. Here is a hands-on introduction using Python and Giotto-TDA...',
   'A mathematician''s guide to topological data analysis.',
   (SELECT id FROM categories WHERE slug = 'blog-tech' LIMIT 1),
   ARRAY['math','ml','data-science'], 387);

  INSERT INTO blogs (user_id, title, content, description, category_id, tags, view_count) VALUES
  (u_priya, 'AI regulation tracker — Q1 2026',
   'A comprehensive overview of AI bills moving through parliaments in India, Brazil, Kenya, and the EU. We tracked 47 pieces of legislation. Here is what passed, what died, and what got amended...',
   'Tracking AI policy across the Global South and beyond.',
   (SELECT id FROM categories WHERE slug = 'blog-career' LIMIT 1),
   ARRAY['ai','policy','regulation'], 520);

  -- Blog comments on new blogs
  INSERT INTO blog_comments (blog_id, user_id, content)
  SELECT b.id, u_luna, 'The connection between traditional craft and sustainability is so powerful. Thank you for telling this story.'
  FROM blogs b WHERE b.title LIKE '%slow fashion%' LIMIT 1;
  INSERT INTO blog_comments (blog_id, user_id, content)
  SELECT b.id, u_morgan, 'Really curious about the mapper algorithm. Added Giotto-TDA to my reading list.'
  FROM blogs b WHERE b.title LIKE '%topology%' LIMIT 1;
  INSERT INTO blog_comments (blog_id, user_id, content)
  SELECT b.id, u_diego, 'That AI regulation piece is essential reading. Shared with our legal team.'
  FROM blogs b WHERE b.title LIKE '%AI regulation%' LIMIT 1;

  -- ── Forums (section → forum → thread → replies) ──────────────────────────
  INSERT INTO forum_sections (name, description) VALUES
  ('Product', 'Product discussions and feedback')
  RETURNING id INTO sec_id;

  INSERT INTO forums (section_id, name, description, thread_count, last_post_at) VALUES
  (sec_id, 'Announcements', 'Release notes and housekeeping', 1, NOW() - interval '30 minutes')
  RETURNING id INTO frm_id;

  INSERT INTO forum_threads (forum_id, user_id, title, content, view_count, reply_count, last_reply_at) VALUES
  (frm_id, u_alex, 'Welcome to the seeded forum', 'Use this thread to test replies and quotes.', 34, 2, NOW() - interval '1 hour')
  RETURNING id INTO th_id;

  INSERT INTO forum_replies (thread_id, user_id, content) VALUES
  (th_id, u_diego, 'Great to have a dedicated space for this.'),
  (th_id, u_riley, '+1 — will post our roadmap here next week.');

  -- More forum threads
  INSERT INTO forum_threads (forum_id, user_id, title, content, view_count, reply_count, last_reply_at) VALUES
  (frm_id, u_morgan, 'Proposal: API versioning policy', 'We should document a formal deprecation window for /v1 routes. My suggestion: 6 months notice before sunsetting any endpoint.', 67, 3, NOW() - interval '3 hours')
  RETURNING id INTO th_id;
  INSERT INTO forum_replies (thread_id, user_id, content) VALUES
  (th_id, u_alex, '+1. I would also add that breaking changes to request/response shapes need a changelog entry 30 days in advance.'),
  (th_id, u_priya, 'As someone building on the API — yes please. Clear deprecation headers would be amazing.'),
  (th_id, u_diego, 'Added to the eng meeting agenda for next Tuesday.');

  INSERT INTO forum_threads (forum_id, user_id, title, content, view_count, reply_count, last_reply_at) VALUES
  (frm_id, u_viktor, 'Interesting bug: floating-point determinism across compilers', 'We hit a reproducibility issue where the same Rust code compiled with different LLVM versions produced slightly different floating-point results. This is a known LLVM issue — here is our mitigation...', 89, 2, NOW() - interval '5 hours')
  RETURNING id INTO th_id;
  INSERT INTO forum_replies (thread_id, user_id, content) VALUES
  (th_id, u_alex, 'Fascinating edge case. Were you able to isolate it to specific LLVM optimization passes?'),
  (th_id, u_viktor, 'Yes — the reassociate pass was the culprit. Disabling it with `-C llvm-args=-reassociate=0` fixed it, at a ~3% perf cost.');

  -- ── Jobs + application ───────────────────────────────────────────────────
  INSERT INTO jobs (user_id, page_id, title, description, location, lat, lng, salary_min, salary_max, salary_period, job_type, category_id, status, currency) VALUES
  (u_diego, (SELECT id FROM pages WHERE page_name = 'techcorp_sf' LIMIT 1),
   'Senior Rust Engineer', 'Realtime infra, NATS, PostgreSQL. Remote-friendly.', 'Remote / SF', 37.78, -122.40,
   150000, 190000, 'yearly', 'full_time', (SELECT id FROM categories WHERE slug = 'job-software' LIMIT 1), 'active', 'USD')
  RETURNING id INTO job_id;

  INSERT INTO job_applications (job_id, user_id, cover_letter, status) VALUES
  (job_id, u_taylor, 'Rust since 2019; strong docs + on-call experience.', 'pending')
  ON CONFLICT DO NOTHING;
  INSERT INTO job_applications (job_id, user_id, cover_letter, status) VALUES
  (job_id, u_viktor, 'Systems programming background; interested in real-time infra.', 'pending')
  ON CONFLICT DO NOTHING;

  INSERT INTO jobs (user_id, title, description, location, job_type, category_id, status) VALUES
  (u_zoe, 'Relief Veterinarian — weekend shifts', 'Small animal clinic; mentorship available.', 'Portland OR', 'part_time',
   (SELECT id FROM categories WHERE slug = 'job-healthcare' LIMIT 1), 'active');

  INSERT INTO jobs (user_id, page_id, title, description, location, lat, lng, job_type, category_id, status) VALUES
  (u_olivia, (SELECT id FROM pages WHERE page_name = 'olivia_studio' LIMIT 1),
   'Junior UX Designer (remote EU)', 'Join our small design team. Figma, user research, and prototyping. 1–2 years experience.', 'Berlin / Remote', 52.52, 13.40,
   'full_time', (SELECT id FROM categories WHERE slug = 'job-design' LIMIT 1), 'active');

  INSERT INTO job_applications (job_id, user_id, cover_letter, status)
  SELECT j.id, u_leila, 'Architect transitioning to UX design; strong visual and spatial design skills.', 'pending'
  FROM jobs j WHERE j.title = 'Junior UX Designer (remote EU)' LIMIT 1
  ON CONFLICT DO NOTHING;

  -- ── Marketplace: products, order, review ─────────────────────────────────
  INSERT INTO products (user_id, page_id, name, description, category_id, price, currency, location, condition, type, status, media, units) VALUES
  (u_luna, (SELECT id FROM pages WHERE page_name = 'luna_gallery' LIMIT 1),
   'Limited print: Midnight Bloom', 'Archival giclée, signed edition of 50.',
   (SELECT id FROM categories WHERE slug = 'product-handmade' LIMIT 1), 129.00, 'USD', 'Milan', 'new', 'sell', 'active',
   '[{"url":"https://images.unsplash.com/photo-1579783902614-a3fb3927b6a5?w=800","type":"image"}]'::jsonb, 12)
  RETURNING id INTO prod_id;

  INSERT INTO orders (buyer_id, seller_id, product_id, quantity, total_price, status) VALUES
  (2, u_luna, prod_id, 1, 129.00, 'completed');

  INSERT INTO product_reviews (product_id, user_id, rating, text) VALUES
  (prod_id, 2, 5, 'Gorgeous print; shipped flat with care.')
  ON CONFLICT DO NOTHING;
  INSERT INTO product_reviews (product_id, user_id, rating, text) VALUES
  (prod_id, u_fatima, 5, 'The colors are even more vivid in person. Taking pride of place in my studio!')
  ON CONFLICT DO NOTHING;
  INSERT INTO product_reviews (product_id, user_id, rating, text) VALUES
  (prod_id, u_leila, 4, 'Beautiful piece. Only wish larger sizes were available.')
  ON CONFLICT DO NOTHING;

  INSERT INTO products (user_id, name, description, category_id, price, currency, condition, type, status, media) VALUES
  (u_blake, 'Smart dimmer bundle (3-pack)', 'Zigbee, works with common hubs.', (SELECT id FROM categories WHERE slug = 'product-electronics' LIMIT 1),
   79.99, 'USD', 'new', 'sell', 'active',
   '[{"url":"https://images.unsplash.com/photo-1558002038-1055907df827?w=800","type":"image"}]'::jsonb);

  -- More products
  INSERT INTO products (user_id, name, description, category_id, price, currency, location, condition, type, status, media, units) VALUES
  (u_mateo, 'Small-batch fermented hot sauce (3-pack)', 'Aged 90 days: mango-habanero, pineapple-scotch bonnet, and ají amarillo.',
   (SELECT id FROM categories WHERE slug = 'product-handmade' LIMIT 1), 24.99, 'USD', 'Medellín', 'new', 'sell', 'active',
   '[{"url":"https://images.unsplash.com/photo-1504674900247-0877df9cc836?w=800","type":"image"}]'::jsonb, 25);

  INSERT INTO products (user_id, name, description, category_id, price, currency, condition, type, status, media) VALUES
  (u_hugo, 'Pixel Forest — Original Soundtrack', '24-track OST from the game. Chiptune meets orchestral. Lossless + MP3 download.',
   (SELECT id FROM categories WHERE slug = 'product-handmade' LIMIT 1), 9.99, 'USD', 'new', 'sell', 'active',
   '[{"url":"https://images.unsplash.com/photo-1511512578047-dfb367046420?w=800","type":"image"}]'::jsonb);

  -- Orders on new products (admin buying from Mateo)
  INSERT INTO orders (buyer_id, seller_id, product_id, quantity, total_price, status)
  SELECT 2, u_mateo, p.id, 2, 49.98, 'completed'
  FROM products p WHERE p.user_id = u_mateo AND p.name LIKE '%hot sauce%' LIMIT 1;

  -- ── Funding + donations ──────────────────────────────────────────────────
  INSERT INTO fundings (user_id, title, description, goal_amount, raised_amount, image) VALUES
  (u_nina, 'Coral nursery expansion', 'Help us plant 5000 coral fragments this season.',
   25000.00, 8200.00, 'https://images.unsplash.com/photo-1544551763-46a013bb70d5?w=800')
  RETURNING id INTO fund_id;

  INSERT INTO funding_donations (funding_id, user_id, amount) VALUES
  (fund_id, 2, 100.00),
  (fund_id, u_emma, 50.00);

  -- More donations from new users
  INSERT INTO funding_donations (funding_id, user_id, amount) VALUES
  (fund_id, u_isabella, 75.00),
  (fund_id, u_yuki, 30.00),
  (fund_id, u_mateo, 40.00),
  (fund_id, u_noah, 60.00)
  ON CONFLICT DO NOTHING;

  -- ── Offers ───────────────────────────────────────────────────────────────
  INSERT INTO offers (user_id, page_id, title, description, image, discount_type, discount_value, currency, expires_at) VALUES
  (u_emma, (SELECT id FROM pages WHERE page_name = 'emmas_kitchen' LIMIT 1),
   'Spring tasting menu', '20% off Sunday seatings this month.',
   'https://images.unsplash.com/photo-1473093295043-cdd812d0e601?w=800',
   'percentage', 20, 'USD', NOW() + interval '30 days');

  INSERT INTO offers (user_id, page_id, title, description, image, discount_type, discount_value, currency, expires_at) VALUES
  (u_luna, (SELECT id FROM pages WHERE page_name = 'luna_gallery' LIMIT 1),
   'Opening night — early collector discount', '15% off all prints purchased during the Liquid Light opening.',
   'https://images.unsplash.com/photo-1579783902614-a3fb3927b6a5?w=800',
   'percentage', 15, 'USD', NOW() + interval '14 days');

  INSERT INTO offers (user_id, page_id, title, description, image, discount_type, discount_value, currency, expires_at) VALUES
  (u_marcus, (SELECT id FROM pages WHERE page_name = 'fit_with_marcus' LIMIT 1),
   'Summer bootcamp — early bird', 'First 20 sign-ups save $50 on the 8-week program.',
   'https://images.unsplash.com/photo-1534438327276-14e5300c3a48?w=800',
   'fixed', 50, 'USD', NOW() + interval '21 days');

  -- ── Movies + games ───────────────────────────────────────────────────────
  INSERT INTO movies (user_id, name, video_url, description, genre, release_year, view_count, is_approved) VALUES
  (u_james, 'Outback sample reel (seed)', 'https://storage.googleapis.com/gtv-videos-bucket/sample/ForBiggerEscapes.mp4',
   'Test movie row for catalog UI.', 'Documentary', 2025, 420, true);

  INSERT INTO movies (user_id, name, video_url, description, genre, release_year, view_count, is_approved) VALUES
  (u_noah, 'Score demo reel — orchestral + electronic', 'https://storage.googleapis.com/gtv-videos-bucket/sample/ForBiggerBlazes.mp4',
   'Selected cues from recent film scoring projects.', 'Soundtrack', 2025, 280, true);

  INSERT INTO games (name, avatar, link, active, player_count) VALUES
  ('Sky Pixels', 'https://images.unsplash.com/photo-1550745165-9bc0b252726f?w=200', 'https://example.com/games/sky-pixels', true, 4);

  INSERT INTO games (name, avatar, link, active, player_count) VALUES
  ('Chess Royale', 'https://images.unsplash.com/photo-1529699211952-734e80c4d42d?w=200', 'https://example.com/games/chess-royale', true, 7);

  INSERT INTO game_players (game_id, user_id)
  SELECT g.id, u_hugo FROM games g WHERE g.name = 'Chess Royale' LIMIT 1
  ON CONFLICT DO NOTHING;
  INSERT INTO game_players (game_id, user_id)
  SELECT g.id, u_sky FROM games g WHERE g.name = 'Chess Royale' LIMIT 1
  ON CONFLICT DO NOTHING;
  INSERT INTO game_players (game_id, user_id)
  SELECT g.id, 2 FROM games g WHERE g.name = 'Chess Royale' LIMIT 1
  ON CONFLICT DO NOTHING;

  INSERT INTO game_players (game_id, user_id)
  SELECT g.id, u_sky FROM games g WHERE g.name = 'Sky Pixels' LIMIT 1
  ON CONFLICT DO NOTHING;
  INSERT INTO game_players (game_id, user_id)
  SELECT g.id, u_morgan FROM games g WHERE g.name = 'Sky Pixels' LIMIT 1
  ON CONFLICT DO NOTHING;
  INSERT INTO game_players (game_id, user_id)
  SELECT g.id, 2 FROM games g WHERE g.name = 'Sky Pixels' LIMIT 1
  ON CONFLICT DO NOTHING;

  -- ── Hashtags + post links ────────────────────────────────────────────────
  INSERT INTO hashtags (tag, use_count, trending) VALUES
  ('wowonderseed', 25, true),
  ('jungleqa', 8, false),
  ('barcelona', 40, true)
  ON CONFLICT (tag) DO NOTHING;

  INSERT INTO post_hashtags (post_id, hashtag_id)
  SELECT s.id, (SELECT id FROM hashtags WHERE tag = 'wowonderseed' LIMIT 1)
  FROM (SELECT id FROM posts WHERE user_id = u_sophia ORDER BY created_at DESC LIMIT 1) s
  ON CONFLICT DO NOTHING;

  INSERT INTO post_hashtags (post_id, hashtag_id)
  SELECT s.id, (SELECT id FROM hashtags WHERE tag = 'jungleqa' LIMIT 1)
  FROM (SELECT id FROM posts WHERE user_id = u_alex ORDER BY created_at DESC LIMIT 1) s
  ON CONFLICT DO NOTHING;

  -- ── Promoted post ad (feed injection) ─────────────────────────────────────
  INSERT INTO user_ads (user_id, ad_type, target_id, name, headline, description, status, budget, placement, impressions)
  SELECT u_diego, 'post', (SELECT id FROM posts WHERE user_id = u_diego AND content LIKE '%Series A%' ORDER BY id DESC LIMIT 1),
   'Seed boost', 'Startup milestone', 'Test sponsored slot', 'active', 100.00, 'feed', 0
  WHERE NOT EXISTS (SELECT 1 FROM user_ads WHERE name = 'Seed boost' AND user_id = u_diego);

  INSERT INTO site_config (category, key, value, value_type)
  VALUES ('seed', 'wowonder_sql_demo_v1', '1', 'string')
  ON CONFLICT (category, key) DO NOTHING;

  END IF;

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
  INSERT INTO cronjob_runs (name, status, message, duration_ms, ran_at)
  SELECT 'clean_expired_stories', 'healthy', 'Removed 12 expired stories', 342, NOW() - interval '1 hour'
  WHERE NOT EXISTS (SELECT 1 FROM cronjob_runs WHERE name = 'clean_expired_stories' AND message = 'Removed 12 expired stories');
  INSERT INTO cronjob_runs (name, status, message, duration_ms, ran_at)
  SELECT 'digest_weekly_memories', 'healthy', 'Dispatched 48 digest emails', 1284, NOW() - interval '6 hours'
  WHERE NOT EXISTS (SELECT 1 FROM cronjob_runs WHERE name = 'digest_weekly_memories' AND message = 'Dispatched 48 digest emails');
  INSERT INTO cronjob_runs (name, status, message, duration_ms, ran_at)
  SELECT 'publish_scheduled_posts', 'healthy', 'Published 3 scheduled posts', 89, NOW() - interval '15 minutes'
  WHERE NOT EXISTS (SELECT 1 FROM cronjob_runs WHERE name = 'publish_scheduled_posts' AND message = 'Published 3 scheduled posts');
  INSERT INTO cronjob_runs (name, status, message, duration_ms, ran_at)
  SELECT 'delete_old_messages', 'warning', 'retention disabled via site_config', 2, NOW() - interval '30 minutes'
  WHERE NOT EXISTS (SELECT 1 FROM cronjob_runs WHERE name = 'delete_old_messages' AND message = 'retention disabled via site_config');

  -- ── Admin audit log: a representative entry ─────────────────────────────
  INSERT INTO admin_audit_log
    (admin_user_id, action, resource_type, resource_id, endpoint, status,
     changes, ip_address, user_agent, created_at)
  SELECT 2, 'PUT', 'settings', 'site', '/v1/admin/settings/site', 200,
     '{"site_title":"Jungle"}'::jsonb, '127.0.0.1'::inet,
     'jungle-admin/1.0', NOW() - interval '10 minutes'
  WHERE NOT EXISTS (
    SELECT 1 FROM admin_audit_log
    WHERE admin_user_id = 2 AND endpoint = '/v1/admin/settings/site' AND action = 'PUT' AND resource_type = 'settings'
    LIMIT 1
  );

  RAISE NOTICE 'Seed pass complete (idempotent: demo bulk skipped if site_config seed.wowonder_sql_demo_v1 exists).';
  RAISE NOTICE 'Users upsert via ON CONFLICT(username); follows/reactions/storage use DO NOTHING / NOT EXISTS where applicable.';
END $$;

COMMIT;
