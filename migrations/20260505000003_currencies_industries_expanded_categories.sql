-- Migration: Currencies + Industries lookups + expanded categories/subcategories
-- Extends the lookups table with currency codes and industry types.
-- Adds more categories and subcategories beyond migration #2.

-- ═══════════════════════════════════════════════════════════════════════════
-- New lookup types: currency, industry, genre
-- ═══════════════════════════════════════════════════════════════════════════

INSERT INTO lookups (lookup_type, value, label_key, icon, sort_order)
SELECT v.lookup_type, v.value, v.label_key, v.icon, v.sort_order
FROM (VALUES
    -- World currencies (~30 major trading currencies)
    ('currency', 'USD', 'lookups.currency.usd', 'dollar-sign',    1),
    ('currency', 'EUR', 'lookups.currency.eur', 'euro',            2),
    ('currency', 'GBP', 'lookups.currency.gbp', 'pound-sterling',  3),
    ('currency', 'JPY', 'lookups.currency.jpy', 'yen',             4),
    ('currency', 'CAD', 'lookups.currency.cad', 'dollar-sign',     5),
    ('currency', 'AUD', 'lookups.currency.aud', 'dollar-sign',     6),
    ('currency', 'CHF', 'lookups.currency.chf', 'swiss-franc',     7),
    ('currency', 'CNY', 'lookups.currency.cny', 'currency',        8),
    ('currency', 'INR', 'lookups.currency.inr', 'rupee',           9),
    ('currency', 'MXN', 'lookups.currency.mxn', 'dollar-sign',    10),
    ('currency', 'BRL', 'lookups.currency.brl', 'dollar-sign',    11),
    ('currency', 'KRW', 'lookups.currency.krw', 'won',            12),
    ('currency', 'SEK', 'lookups.currency.sek', 'swedish-krona',  13),
    ('currency', 'NOK', 'lookups.currency.nok', 'norwegian-krone', 14),
    ('currency', 'DKK', 'lookups.currency.dkk', 'danish-krone',   15),
    ('currency', 'SGD', 'lookups.currency.sgd', 'dollar-sign',    16),
    ('currency', 'HKD', 'lookups.currency.hkd', 'dollar-sign',    17),
    ('currency', 'NZD', 'lookups.currency.nzd', 'dollar-sign',    18),
    ('currency', 'TRY', 'lookups.currency.try', 'turkish-lira',   19),
    ('currency', 'RUB', 'lookups.currency.rub', 'russian-ruble',  20),
    ('currency', 'ZAR', 'lookups.currency.zar', 'south-african-rand', 21),
    ('currency', 'ARS', 'lookups.currency.ars', 'dollar-sign',    22),
    ('currency', 'CLP', 'lookups.currency.clp', 'dollar-sign',    23),
    ('currency', 'COP', 'lookups.currency.cop', 'dollar-sign',    24),
    ('currency', 'PEN', 'lookups.currency.pen', 'dollar-sign',    25),
    ('currency', 'AED', 'lookups.currency.aed', 'dollar-sign',    26),
    ('currency', 'SAR', 'lookups.currency.sar', 'dollar-sign',    27),
    ('currency', 'NGN', 'lookups.currency.ngn', 'naira',          28),
    ('currency', 'EGP', 'lookups.currency.egp', 'egyptian-pound', 29),
    ('currency', 'PHP', 'lookups.currency.php', 'dollar-sign',    30),

    -- Industries (~25)
    ('industry', 'technology',           'lookups.industry.technology',           'cpu',          1),
    ('industry', 'finance',              'lookups.industry.finance',              'landmark',      2),
    ('industry', 'healthcare',           'lookups.industry.healthcare',           'heart-pulse',   3),
    ('industry', 'education',            'lookups.industry.education',            'graduation-cap',4),
    ('industry', 'marketing',            'lookups.industry.marketing',            'megaphone',     5),
    ('industry', 'design',               'lookups.industry.design',               'palette',       6),
    ('industry', 'sales',                'lookups.industry.sales',                'trending-up',   7),
    ('industry', 'engineering',          'lookups.industry.engineering',          'wrench',        8),
    ('industry', 'consulting',           'lookups.industry.consulting',           'briefcase',     9),
    ('industry', 'media',                'lookups.industry.media',                'tv',           10),
    ('industry', 'legal',                'lookups.industry.legal',                'scale',        11),
    ('industry', 'manufacturing',        'lookups.industry.manufacturing',        'factory',      12),
    ('industry', 'retail',               'lookups.industry.retail',               'shopping-cart', 13),
    ('industry', 'real_estate',          'lookups.industry.real_estate',          'building',     14),
    ('industry', 'hospitality',          'lookups.industry.hospitality',          'utensils',     15),
    ('industry', 'transportation',       'lookups.industry.transportation',       'truck',        16),
    ('industry', 'energy',               'lookups.industry.energy',               'zap',          17),
    ('industry', 'agriculture',          'lookups.industry.agriculture',          'sprout',       18),
    ('industry', 'telecommunications',   'lookups.industry.telecommunications',   'phone',        19),
    ('industry', 'gaming',               'lookups.industry.gaming',               'gamepad',      20),
    ('industry', 'pharmaceutical',       'lookups.industry.pharmaceutical',       'pill',         21),
    ('industry', 'insurance',            'lookups.industry.insurance',            'shield',       22),
    ('industry', 'cybersecurity',        'lookups.industry.cybersecurity',        'shield-check', 23),
    ('industry', 'nonprofit',            'lookups.industry.nonprofit',            'heart',        24),
    ('industry', 'government',           'lookups.industry.government',           'landmark',     25),

    -- Movie / content genres (~20) — values match the backend movie API filter
    ('genre', 'Action',        'lookups.genre.action',          'flame',       1),
    ('genre', 'Comedy',        'lookups.genre.comedy',          'laugh',       2),
    ('genre', 'Drama',         'lookups.genre.drama',           'drama',       3),
    ('genre', 'Horror',        'lookups.genre.horror',          'ghost',       4),
    ('genre', 'Thriller',      'lookups.genre.thriller',        'eye',         5),
    ('genre', 'Romance',       'lookups.genre.romance',         'heart',       6),
    ('genre', 'Sci-Fi',        'lookups.genre.sci_fi',          'rocket',      7),
    ('genre', 'Animation',     'lookups.genre.animation',       'clapperboard',8),
    ('genre', 'Documentary',   'lookups.genre.documentary',     'book-open',   9),
    ('genre', 'Fantasy',       'lookups.genre.fantasy',         'wand',       10),
    ('genre', 'Mystery',       'lookups.genre.mystery',         'search',     11),
    ('genre', 'Crime',         'lookups.genre.crime',           'shield-alert',12),
    ('genre', 'Adventure',     'lookups.genre.adventure',       'compass',    13),
    ('genre', 'Musical',       'lookups.genre.musical',         'music',      14),
    ('genre', 'War',           'lookups.genre.war',             'swords',     15),
    ('genre', 'Western',       'lookups.genre.western',         'sun',        16),
    ('genre', 'Biography',     'lookups.genre.biography',       'user',       17),
    ('genre', 'History',       'lookups.genre.history',         'landmark',   18),
    ('genre', 'Family',        'lookups.genre.family',          'users',      19),
    ('genre', 'Sport',         'lookups.genre.sport',           'trophy',     20)
) AS v(lookup_type, value, label_key, icon, sort_order)
WHERE NOT EXISTS (SELECT 1 FROM lookups l WHERE l.lookup_type = v.lookup_type AND l.value = v.value);

-- ═══════════════════════════════════════════════════════════════════════════
-- Expanded product categories (+25 new ones beyond the 48 in migration #2)
-- ═══════════════════════════════════════════════════════════════════════════

INSERT INTO categories (type, name_key, slug, active, sort_order)
SELECT v.type, v.name_key, v.slug, v.active, v.sort_order
FROM (VALUES
    ('product', 'Electronics',              'electronics',               true, 10),
    ('product', 'Clothing & Fashion',       'clothing-fashion',          true, 20),
    ('product', 'Furniture & Home',         'furniture-home',            true, 30),
    ('product', 'Cars & Vehicles',          'cars-vehicles',             true, 40),
    ('product', 'Books & Magazines',        'books-magazines',           true, 50),
    ('product', 'Sports & Outdoors',        'sports-outdoors',           true, 60),
    ('product', 'Toys & Games',             'toys-games',                true, 70),
    ('product', 'Musical Instruments',      'musical-instruments',       true, 80),
    ('product', 'Tools & Hardware',         'tools-hardware',            true, 90),
    ('product', 'Garden & Outdoor',         'garden-outdoor',            true, 100),
    ('product', 'Pet Supplies',             'pet-supplies',              true, 110),
    ('product', 'Baby & Kids',              'baby-kids',                 true, 120),
    ('product', 'Jewelry & Watches',        'jewelry-watches',           true, 130),
    ('product', 'Bags & Luggage',           'bags-luggage',              true, 140),
    ('product', 'Shoes & Footwear',         'shoes-footwear',            true, 150),
    ('product', 'Health & Beauty',          'health-beauty',             true, 160),
    ('product', 'Office Supplies',          'office-supplies',           true, 170),
    ('product', 'Food & Beverages',         'food-beverages',            true, 180),
    ('product', 'Antiques & Collectibles',  'antiques-collectibles',     true, 190),
    ('product', 'Art & Crafts',             'art-crafts',                true, 200),
    ('product', 'Video Games & Consoles',   'video-games-consoles',      true, 210),
    ('product', 'Smartphones & Tablets',    'smartphones-tablets',       true, 220),
    ('product', 'Laptops & Computers',      'laptops-computers',         true, 230),
    ('product', 'Cameras & Photo',          'cameras-photo',             true, 240),
    ('product', 'TVs & Home Theater',       'tvs-home-theater',          true, 250),
    ('product', 'Audio & Headphones',       'audio-headphones',          true, 260),
    ('product', 'Bicycles & Scooters',      'bicycles-scooters',         true, 270),
    ('product', 'Camping & Hiking',         'camping-hiking',            true, 280),
    ('product', 'Fishing & Hunting',        'fishing-hunting',           true, 290),
    ('product', 'Fitness Equipment',        'fitness-equipment',         true, 300),
    ('product', 'Home Decor',               'home-decor',                true, 310),
    ('product', 'Kitchen Appliances',       'kitchen-appliances',        true, 320),
    ('product', 'Real Estate',              'real-estate',               true, 330),
    ('product', 'Tickets & Events',         'tickets-events',            true, 340),
    ('product', 'Industrial Equipment',     'industrial-equipment',      true, 350),
    ('product', 'Medical Supplies',         'medical-supplies',          true, 360),
    ('product', 'Agriculture & Farming',    'agriculture-farming',       true, 370),
    ('product', 'Board Games & Puzzles',    'board-games-puzzles',       true, 380),
    ('product', 'DIY & Materials',          'diy-materials',             true, 390),
    ('product', 'Vintage & Retro',          'vintage-retro',             true, 400),
    ('product', 'Cosplay & Costumes',       'cosplay-costumes',          true, 410),
    ('product', 'Party Supplies',           'party-supplies',            true, 420),
    ('product', 'Printers & Scanners',      'printers-scanners',         true, 430),
    ('product', 'Networking Equipment',     'networking-equipment',      true, 440),
    ('product', 'Software & Apps',          'software-apps',             true, 450),
    ('product', 'Services',                 'services',                  true, 460),
    ('product', 'Free Stuff',               'free-stuff',                true, 470),
    ('product', 'Other',                    'other',                     true, 999),
    -- NEW product categories
    ('product', 'Home Appliances',          'home-appliances',           true, 55),
    ('product', 'Wearable Tech',            'wearable-tech',             true, 115),
    ('product', 'Collectible Cards',        'collectible-cards',         true, 195),
    ('product', '3D Printing & CNC',        '3d-printing-cnc',           true, 235),
    ('product', 'Drones & Robotics',        'drones-robotics',           true, 255),
    ('product', 'Electric Vehicles & E-Bikes','electric-vehicles-ebikes', true, 275),
    ('product', 'Motorsports Equipment',    'motorsports-equipment',     true, 285),
    ('product', 'Home Brewing & Winemaking','home-brewing-winemaking',   true, 325),
    ('product', 'Smart Home & Domotics',    'smart-home-domotics',       true, 345),
    ('product', 'Laboratory Equipment',     'laboratory-equipment',      true, 355),
    ('product', 'Uniforms & Workwear',      'uniforms-workwear',         true, 400),
    ('product', 'Gravestone & Memorial',    'gravestone-memorial',       true, 415),
    ('product', 'Wedding Supplies',         'wedding-supplies',          true, 425),
    ('product', 'Surveillance & Security',  'surveillance-security',     true, 435),
    ('product', 'Generators & Power',       'generators-power',          true, 445),
    ('product', 'Tickets & Vouchers',       'tickets-vouchers',          true, 455),
    ('product', 'Religious Items',          'religious-items',           true, 475),
    ('product', 'Tobacco & Vaping',         'tobacco-vaping',            true, 480),
    ('product', 'Crypto & Mining Hardware', 'crypto-mining-hardware',    true, 485),
    ('product', 'Mobility & Accessibility', 'mobility-accessibility',    true, 490),
    ('product', 'Fireworks & Pyrotechnics', 'fireworks-pyrotechnics',    true, 495),
    ('product', 'Trophies & Awards',        'trophies-awards',           true, 500),
    ('product', 'Flags & Banners',          'flags-banners',             true, 505),
    ('product', 'Stamps & Philately',       'stamps-philately',          true, 510),
    ('product', 'Coins & Numismatics',      'coins-numismatics',         true, 515)
) AS v(type, name_key, slug, active, sort_order)
WHERE NOT EXISTS (SELECT 1 FROM categories c WHERE c.slug = v.slug AND c.type = v.type);

-- ═══════════════════════════════════════════════════════════════════════════
-- Expanded subcategories for NEW product categories
-- ═══════════════════════════════════════════════════════════════════════════

DO $$
DECLARE
    cat_id BIGINT;
BEGIN
    -- Home Appliances
    SELECT id INTO cat_id FROM categories WHERE slug = 'home-appliances' AND type = 'product';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Refrigerators & Freezers', 'product'),
            ('Washing Machines', 'product'),
            ('Dryers', 'product'),
            ('Dishwashers', 'product'),
            ('Vacuum Cleaners', 'product'),
            ('Irons & Steamers', 'product'),
            ('Water Heaters', 'product'),
            ('Air Purifiers & Humidifiers', 'product')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    -- Wearable Tech
    SELECT id INTO cat_id FROM categories WHERE slug = 'wearable-tech' AND type = 'product';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Smartwatches', 'product'),
            ('Fitness Trackers', 'product'),
            ('VR & AR Headsets', 'product'),
            ('Smart Glasses', 'product'),
            ('Smart Rings', 'product'),
            ('Health Monitors', 'product'),
            ('Wearable Cameras', 'product'),
            ('Smart Clothing', 'product')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    -- Smart Home & Domotics
    SELECT id INTO cat_id FROM categories WHERE slug = 'smart-home-domotics' AND type = 'product';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Smart Lights & Bulbs', 'product'),
            ('Smart Plugs & Outlets', 'product'),
            ('Smart Locks', 'product'),
            ('Smart Thermostats', 'product'),
            ('Video Doorbells', 'product'),
            ('Smart Speakers & Hubs', 'product'),
            ('Sensors & Detectors', 'product'),
            ('Smart Blinds & Curtains', 'product')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    -- Drones & Robotics
    SELECT id INTO cat_id FROM categories WHERE slug = 'drones-robotics' AND type = 'product';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Camera Drones', 'product'),
            ('Racing Drones', 'product'),
            ('Toy Drones', 'product'),
            ('Agricultural Drones', 'product'),
            ('Robot Kits', 'product'),
            ('Robot Vacuums', 'product'),
            ('Drone Accessories', 'product'),
            ('FPV Goggles & Gear', 'product')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    -- Wedding Supplies
    SELECT id INTO cat_id FROM categories WHERE slug = 'wedding-supplies' AND type = 'product';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Wedding Dresses', 'product'),
            ('Wedding Rings & Bands', 'product'),
            ('Wedding Decorations', 'product'),
            ('Invitations & Stationery', 'product'),
            ('Bridal Accessories', 'product'),
            ('Groomsmen Gifts', 'product'),
            ('Centerpieces', 'product'),
            ('Favors & Gifts', 'product')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    -- Electric Vehicles & E-Bikes
    SELECT id INTO cat_id FROM categories WHERE slug = 'electric-vehicles-ebikes' AND type = 'product';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Electric Cars', 'product'),
            ('Electric Scooters', 'product'),
            ('Electric Bikes', 'product'),
            ('Electric Skateboards', 'product'),
            ('Hoverboards & Segways', 'product'),
            ('EV Charging Stations', 'product'),
            ('Electric Motorcycles', 'product'),
            ('E-Bike Conversion Kits', 'product')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    -- 3D Printing & CNC
    SELECT id INTO cat_id FROM categories WHERE slug = '3d-printing-cnc' AND type = 'product';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('3D Printers', 'product'),
            ('Filament & Resin', 'product'),
            ('CNC Machines', 'product'),
            ('Laser Engravers', 'product'),
            ('3D Scanner', 'product'),
            ('Printer Parts & Upgrades', 'product'),
            ('3D Models & Files', 'product'),
            ('Printing Services', 'product')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    -- Surveillance & Security
    SELECT id INTO cat_id FROM categories WHERE slug = 'surveillance-security' AND type = 'product';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Security Cameras', 'product'),
            ('Alarm Systems', 'product'),
            ('Door Locks & Access Control', 'product'),
            ('Motion Sensors', 'product'),
            ('NVR & DVR Systems', 'product'),
            ('Intercom Systems', 'product'),
            ('Burglar Alarms', 'product'),
            ('Fire & Smoke Detectors', 'product')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    -- Collectible Cards
    SELECT id INTO cat_id FROM categories WHERE slug = 'collectible-cards' AND type = 'product';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Pokemon Cards', 'product'),
            ('Magic: The Gathering', 'product'),
            ('Yu-Gi-Oh! Cards', 'product'),
            ('Sports Cards', 'product'),
            ('Card Accessories & Sleeves', 'product'),
            ('Graded & PSA Cards', 'product'),
            ('Booster Boxes & Packs', 'product'),
            ('Card Binders & Albums', 'product')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    -- Home Brewing & Winemaking
    SELECT id INTO cat_id FROM categories WHERE slug = 'home-brewing-winemaking' AND type = 'product';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Beer Brewing Kits', 'product'),
            ('Wine Making Kits', 'product'),
            ('Fermentation Equipment', 'product'),
            ('Bottling & Kegging', 'product'),
            ('Grains & Malt', 'product'),
            ('Hops', 'product'),
            ('Yeast & Cultures', 'product'),
            ('Distilling Equipment', 'product')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    -- Generators & Power
    SELECT id INTO cat_id FROM categories WHERE slug = 'generators-power' AND type = 'product';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Portable Generators', 'product'),
            ('Standby Generators', 'product'),
            ('Solar Generators', 'product'),
            ('Inverter Generators', 'product'),
            ('UPS & Battery Backup', 'product'),
            ('Transfer Switches', 'product'),
            ('Generator Parts', 'product'),
            ('Extension Cords & Cables', 'product')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    -- Additional existing categories — subcategories for ones not covered in migration #2

    SELECT id INTO cat_id FROM categories WHERE slug = 'bicycles-scooters' AND type = 'product';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Mountain Bikes', 'product'),
            ('Road Bikes', 'product'),
            ('Hybrid Bikes', 'product'),
            ('Kids Bikes', 'product'),
            ('Scooters', 'product'),
            ('Bike Parts & Components', 'product'),
            ('Bike Helmets & Safety', 'product'),
            ('Bike Accessories', 'product')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'fitness-equipment' AND type = 'product';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Treadmills', 'product'),
            ('Dumbbells & Free Weights', 'product'),
            ('Home Gyms & Racks', 'product'),
            ('Resistance Bands', 'product'),
            ('Exercise Bikes', 'product'),
            ('Rowing Machines', 'product'),
            ('Kettlebells', 'product'),
            ('Yoga Mats & Accessories', 'product')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'office-supplies' AND type = 'product';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Paper & Notebooks', 'product'),
            ('Pens & Writing', 'product'),
            ('Desk Accessories', 'product'),
            ('Filing & Storage', 'product'),
            ('Whiteboards & Corkboards', 'product'),
            ('Staplers & Hole Punchers', 'product'),
            ('Envelopes & Mailing', 'product'),
            ('Calendars & Planners', 'product')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'food-beverages' AND type = 'product';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Snacks & Chips', 'product'),
            ('Coffee & Tea', 'product'),
            ('Organic Food', 'product'),
            ('Spices & Seasonings', 'product'),
            ('Candy & Chocolate', 'product'),
            ('Canned & Packaged Food', 'product'),
            ('Beverages & Soft Drinks', 'product'),
            ('Supplements & Protein', 'product')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'tools-hardware' AND type = 'product';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Power Tools', 'product'),
            ('Hand Tools', 'product'),
            ('Measuring & Layout', 'product'),
            ('Fasteners & Hardware', 'product'),
            ('Paint & Supplies', 'product'),
            ('Ladders & Scaffolding', 'product'),
            ('Welding & Soldering', 'product'),
            ('Tool Storage', 'product')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'garden-outdoor' AND type = 'product';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Plants & Seeds', 'product'),
            ('Pots & Planters', 'product'),
            ('Garden Tools', 'product'),
            ('Lawn Mowers', 'product'),
            ('Grills & BBQs', 'product'),
            ('Outdoor Lighting', 'product'),
            ('Patio Furniture', 'product'),
            ('Fencing & Gates', 'product')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'tickets-events' AND type = 'product';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Concert Tickets', 'product'),
            ('Sports Tickets', 'product'),
            ('Theater & Shows', 'product'),
            ('Festival Passes', 'product'),
            ('Conference & Workshop', 'product'),
            ('Movie Tickets', 'product'),
            ('Season Passes', 'product'),
            ('Event Planning Services', 'product')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'services' AND type = 'product';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Home Repair', 'product'),
            ('Auto Repair', 'product'),
            ('Tech Support', 'product'),
            ('Tutoring & Lessons', 'product'),
            ('Cleaning Services', 'product'),
            ('Moving & Hauling', 'product'),
            ('Pet Sitting & Walking', 'product'),
            ('Freelance & Consulting', 'product')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    -- ── Additional blog subcategories for categories that didn't have them ──

    SELECT id INTO cat_id FROM categories WHERE slug = 'lifestyle' AND type = 'blog';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Minimalism', 'blog'),
            ('Slow Living', 'blog'),
            ('Urban Living', 'blog'),
            ('Rural & Homesteading', 'blog'),
            ('Interior Design & Decor', 'blog'),
            ('Personal Finance', 'blog'),
            ('Hobbies & Interests', 'blog'),
            ('Self-Improvement', 'blog')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'sports' AND type = 'blog';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Football / Soccer', 'blog'),
            ('Basketball', 'blog'),
            ('Tennis', 'blog'),
            ('Martial Arts & Combat', 'blog'),
            ('Extreme Sports', 'blog'),
            ('Running & Marathon', 'blog'),
            ('Cycling', 'blog'),
            ('Fantasy Sports', 'blog')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'fashion-style' AND type = 'blog';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Streetwear', 'blog'),
            ('Luxury Fashion', 'blog'),
            ('Sustainable Fashion', 'blog'),
            ('Vintage Style', 'blog'),
            ('Men''s Fashion', 'blog'),
            ('Women''s Fashion', 'blog'),
            ('Accessories & Jewelry', 'blog'),
            ('Seasonal Trends', 'blog')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'gaming' AND type = 'blog';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('PC Gaming', 'blog'),
            ('Console News', 'blog'),
            ('Mobile Gaming', 'blog'),
            ('Indie Games', 'blog'),
            ('Esports & Competitive', 'blog'),
            ('Game Reviews', 'blog'),
            ('Game Development', 'blog'),
            ('Retro Gaming', 'blog')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'music' AND type = 'blog';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Album Reviews', 'blog'),
            ('Artist Interviews', 'blog'),
            ('Music Production', 'blog'),
            ('Gear & Instruments', 'blog'),
            ('Concerts & Festivals', 'blog'),
            ('Vinyl & Collecting', 'blog'),
            ('Music History', 'blog'),
            ('Playlists & Curation', 'blog')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'education-learning' AND type = 'blog';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Online Courses', 'blog'),
            ('Study Tips', 'blog'),
            ('Language Learning', 'blog'),
            ('STEM Education', 'blog'),
            ('Test Prep', 'blog'),
            ('Homeschooling', 'blog'),
            ('College & University', 'blog'),
            ('Professional Certifications', 'blog')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'career-professional' AND type = 'blog';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Resume & Interview Tips', 'blog'),
            ('Remote Work', 'blog'),
            ('Freelancing', 'blog'),
            ('Career Change', 'blog'),
            ('Workplace Culture', 'blog'),
            ('Negotiation & Salary', 'blog'),
            ('Networking', 'blog'),
            ('Leadership Skills', 'blog')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'relationships-dating' AND type = 'blog';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Dating Advice', 'blog'),
            ('Marriage & Long-Term', 'blog'),
            ('Breakups & Divorce', 'blog'),
            ('Friendship', 'blog'),
            ('Family Relationships', 'blog'),
            ('LGBTQ+ Relationships', 'blog'),
            ('Communication Skills', 'blog'),
            ('Self-Love & Single Life', 'blog')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'pets-animals' AND type = 'blog';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Dog Care & Training', 'blog'),
            ('Cat Care & Behavior', 'blog'),
            ('Aquarium & Fish', 'blog'),
            ('Exotic Pets', 'blog'),
            ('Pet Health & Nutrition', 'blog'),
            ('Pet Adoption', 'blog'),
            ('Horse Care & Equestrian', 'blog'),
            ('Wildlife & Conservation', 'blog')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    -- ── Additional group subcategories ──

    SELECT id INTO cat_id FROM categories WHERE slug = 'fitness-wellness' AND type = 'group';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Weightlifting & Bodybuilding', 'group'),
            ('CrossFit Community', 'group'),
            ('Running & Marathon Club', 'group'),
            ('Yoga & Pilates Group', 'group'),
            ('Dance Fitness', 'group'),
            ('Nutrition & Meal Prep', 'group'),
            ('Mental Health Support', 'group'),
            ('Seniors Fitness', 'group')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'foodies-dining' AND type = 'group';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Vegan & Plant-Based', 'group'),
            ('BBQ & Grilling', 'group'),
            ('Baking Enthusiasts', 'group'),
            ('Wine & Spirits Tasting', 'group'),
            ('Street Food Lovers', 'group'),
            ('Restaurant Explorers', 'group'),
            ('Home Chefs & Cooking', 'group'),
            ('Coffee & Tea Culture', 'group')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'book-club-reading' AND type = 'group';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Fiction Book Club', 'group'),
            ('Non-Fiction Book Club', 'group'),
            ('Sci-Fi & Fantasy Readers', 'group'),
            ('Mystery & Thriller Fans', 'group'),
            ('Romance Readers', 'group'),
            ('Comics & Graphic Novels', 'group'),
            ('Poetry Circle', 'group'),
            ('Audiobook Club', 'group')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'entrepreneurship-startups' AND type = 'group';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Early-Stage Founders', 'group'),
            ('Growth-Stage Companies', 'group'),
            ('SaaS Startups', 'group'),
            ('E-Commerce Entrepreneurs', 'group'),
            ('Social Entrepreneurship', 'group'),
            ('Pitch Practice & Feedback', 'group'),
            ('Women Entrepreneurs', 'group'),
            ('Solo Founders', 'group')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    -- ── Additional page subcategories ──

    SELECT id INTO cat_id FROM categories WHERE slug = 'tech-startup' AND type = 'page';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('SaaS Company', 'page'),
            ('Fintech', 'page'),
            ('Healthtech', 'page'),
            ('Edtech', 'page'),
            ('E-Commerce Platform', 'page'),
            ('Mobile App Company', 'page'),
            ('AI / ML Company', 'page'),
            ('Gaming Studio', 'page')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'education-institution' AND type = 'page';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Primary School', 'page'),
            ('High School', 'page'),
            ('College / University', 'page'),
            ('Vocational & Trade School', 'page'),
            ('Language School', 'page'),
            ('Online School', 'page'),
            ('Music & Arts School', 'page'),
            ('Driving School', 'page')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

END $$;
