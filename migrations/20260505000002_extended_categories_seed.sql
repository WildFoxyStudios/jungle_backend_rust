-- Migration: Extended categories + subcategories seed
-- Populates the generic categories table with comprehensive data for:
--   product, job, page, group, blog
-- and adds matching subcategories for top parent categories.

-- Add UNIQUE index to prevent duplicate subcategories
CREATE UNIQUE INDEX IF NOT EXISTS uq_sub_categories_cat_lang ON sub_categories (category_id, lang_key);

-- ── Product categories (50+) ──────────────────────────────────────────

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
    ('product', 'Other',                    'other',                     true, 999)
) AS v(type, name_key, slug, active, sort_order)
WHERE NOT EXISTS (SELECT 1 FROM categories c WHERE c.slug = v.slug AND c.type = v.type);

-- ── Job categories (30+) ──────────────────────────────────────────────

INSERT INTO categories (type, name_key, slug, active, sort_order)
SELECT v.type, v.name_key, v.slug, v.active, v.sort_order
FROM (VALUES
    ('job', 'Software Engineering',         'software-engineering',      true, 10),
    ('job', 'Design & Creative',            'design-creative',           true, 20),
    ('job', 'Marketing & Advertising',      'marketing-advertising',     true, 30),
    ('job', 'Sales & Business Development', 'sales-business-dev',        true, 40),
    ('job', 'Finance & Accounting',         'finance-accounting',        true, 50),
    ('job', 'Healthcare & Medical',         'healthcare-medical',        true, 60),
    ('job', 'Education & Teaching',         'education-teaching',        true, 70),
    ('job', 'Legal & Compliance',           'legal-compliance',          true, 80),
    ('job', 'Construction & Engineering',   'construction-engineering',  true, 90),
    ('job', 'Hospitality & Tourism',        'hospitality-tourism',       true, 100),
    ('job', 'Transportation & Logistics',   'transportation-logistics',  true, 110),
    ('job', 'Retail & Wholesale',           'retail-wholesale',          true, 120),
    ('job', 'Manufacturing & Production',   'manufacturing-production',  true, 130),
    ('job', 'Customer Service & Support',   'customer-service-support',  true, 140),
    ('job', 'Human Resources',              'human-resources',           true, 150),
    ('job', 'Data Science & Analytics',     'data-science-analytics',    true, 160),
    ('job', 'Product Management',           'product-management',        true, 170),
    ('job', 'Operations & Supply Chain',    'operations-supply-chain',   true, 180),
    ('job', 'Consulting & Strategy',        'consulting-strategy',       true, 190),
    ('job', 'Media & Communications',       'media-communications',      true, 200),
    ('job', 'Real Estate',                  'real-estate',               true, 210),
    ('job', 'Government & Public Sector',   'government-public-sector',  true, 220),
    ('job', 'Nonprofit & Social Services',  'nonprofit-social',          true, 230),
    ('job', 'Energy & Utilities',           'energy-utilities',          true, 240),
    ('job', 'Agriculture & Forestry',       'agriculture-forestry',      true, 250),
    ('job', 'Pharmaceutical & Biotech',     'pharma-biotech',            true, 260),
    ('job', 'Insurance',                    'insurance',                 true, 270),
    ('job', 'Telecommunications',           'telecommunications',        true, 280),
    ('job', 'Gaming & Entertainment',       'gaming-entertainment',      true, 290),
    ('job', 'AI & Machine Learning',        'ai-machine-learning',       true, 300),
    ('job', 'Cybersecurity',                'cybersecurity',             true, 310),
    ('job', 'Blockchain & Web3',            'blockchain-web3',           true, 320),
    ('job', 'Environmental & Sustainability','environmental-sustainability', true, 330),
    ('job', 'Other',                        'other',                     true, 999)
) AS v(type, name_key, slug, active, sort_order)
WHERE NOT EXISTS (SELECT 1 FROM categories c WHERE c.slug = v.slug AND c.type = v.type);

-- ── Page categories (40+) ─────────────────────────────────────────────

INSERT INTO categories (type, name_key, slug, active, sort_order)
SELECT v.type, v.name_key, v.slug, v.active, v.sort_order
FROM (VALUES
    ('page', 'Restaurant',                  'restaurant',                true, 10),
    ('page', 'Cafe & Coffee Shop',          'cafe-coffee-shop',          true, 20),
    ('page', 'Retail Store',                'retail-store',              true, 30),
    ('page', 'Salon & Spa',                 'salon-spa',                 true, 40),
    ('page', 'Gym & Fitness Center',        'gym-fitness-center',        true, 50),
    ('page', 'Hotel & Accommodation',       'hotel-accommodation',       true, 60),
    ('page', 'Bar & Nightclub',             'bar-nightclub',             true, 70),
    ('page', 'Automotive Service',          'automotive-service',        true, 80),
    ('page', 'Pet Services',                'pet-services',              true, 90),
    ('page', 'Medical Clinic',              'medical-clinic',            true, 100),
    ('page', 'Dentist',                     'dentist',                   true, 110),
    ('page', 'Law Firm',                    'law-firm',                  true, 120),
    ('page', 'Real Estate Agency',          'real-estate-agency',        true, 130),
    ('page', 'Travel Agency',               'travel-agency',             true, 140),
    ('page', 'Event Planning',              'event-planning',            true, 150),
    ('page', 'Photography Studio',          'photography',               true, 160),
    ('page', 'Art Gallery',                 'art-gallery',               true, 170),
    ('page', 'Music Studio',                'music-studio',              true, 180),
    ('page', 'Dance Studio',                'dance-studio',              true, 190),
    ('page', 'Tutoring & Education',        'tutoring-education',        true, 200),
    ('page', 'Religious Organization',      'religious-org',             true, 210),
    ('page', 'Nonprofit Organization',      'nonprofit',                 true, 220),
    ('page', 'Community Organization',      'community-org',             true, 230),
    ('page', 'Sports Team',                 'sports-team',               true, 240),
    ('page', 'Government Organization',     'government-org',            true, 250),
    ('page', 'Political Organization',      'political-org',             true, 260),
    ('page', 'Entertainment & Media',       'entertainment-media',       true, 270),
    ('page', 'Financial Services',          'financial-services',        true, 280),
    ('page', 'Insurance Agency',            'insurance-agency',          true, 290),
    ('page', 'Home Services',               'home-services',             true, 300),
    ('page', 'Cleaning Services',           'cleaning-services',         true, 310),
    ('page', 'Landscaping & Gardening',     'landscaping-gardening',     true, 320),
    ('page', 'Construction Company',        'construction-company',      true, 330),
    ('page', 'Architecture Firm',           'architecture-firm',         true, 340),
    ('page', 'Interior Design',             'interior-design',           true, 350),
    ('page', 'IT Services & Support',       'it-services',               true, 360),
    ('page', 'Marketing Agency',            'marketing-agency',          true, 370),
    ('page', 'Consulting Firm',             'consulting-firm',           true, 380),
    ('page', 'Education Institution',       'education-institution',     true, 390),
    ('page', 'Fashion & Clothing Store',    'fashion-clothing-store',    true, 400),
    ('page', 'Bakery & Pastry Shop',        'bakery-pastry',             true, 410),
    ('page', 'Tech Startup',                'tech-startup',              true, 420),
    ('page', 'Other',                       'other',                     true, 999)
) AS v(type, name_key, slug, active, sort_order)
WHERE NOT EXISTS (SELECT 1 FROM categories c WHERE c.slug = v.slug AND c.type = v.type);

-- ── Group categories (30+) ────────────────────────────────────────────

INSERT INTO categories (type, name_key, slug, active, sort_order)
SELECT v.type, v.name_key, v.slug, v.active, v.sort_order
FROM (VALUES
    ('group', 'Professional Networking',    'professional-networking',   true, 10),
    ('group', 'Hobby & Special Interest',   'hobby-interest',            true, 20),
    ('group', 'Sports & Recreation',        'sports-recreation',         true, 30),
    ('group', 'Gaming & Esports',           'gaming-esports',            true, 40),
    ('group', 'Music & Bands',              'music-bands',               true, 50),
    ('group', 'Art & Creativity',           'art-creativity',            true, 60),
    ('group', 'Book Club & Reading',        'book-club-reading',         true, 70),
    ('group', 'Food & Cooking',             'food-cooking',              true, 80),
    ('group', 'Travel & Adventure',         'travel-adventure',          true, 90),
    ('group', 'Fitness & Wellness',         'fitness-wellness',          true, 100),
    ('group', 'Parenting & Family',         'parenting-family',          true, 110),
    ('group', 'Language Exchange',          'language-exchange',         true, 120),
    ('group', 'Tech & Coding',              'tech-coding',               true, 130),
    ('group', 'Entrepreneurship & Startups','entrepreneurship-startups', true, 140),
    ('group', 'Investing & Finance',        'investing-finance',         true, 150),
    ('group', 'Environment & Conservation', 'environment-conservation',  true, 160),
    ('group', 'Social Justice & Activism',  'social-justice-activism',   true, 170),
    ('group', 'Volunteering & Community',   'volunteering-community',    true, 180),
    ('group', 'Alumni & School',            'alumni-school',             true, 190),
    ('group', 'Pets & Animals',             'pets-animals',              true, 200),
    ('group', 'Photography & Filmmaking',   'photography-filmmaking',    true, 210),
    ('group', 'Writing & Poetry',           'writing-poetry',            true, 220),
    ('group', 'Dance & Performance',        'dance-performance',         true, 230),
    ('group', 'Yoga & Meditation',          'yoga-meditation',           true, 240),
    ('group', 'Hiking & Outdoor',           'hiking-outdoor',            true, 250),
    ('group', 'Cycling',                    'cycling',                   true, 260),
    ('group', 'Foodies & Dining',           'foodies-dining',            true, 270),
    ('group', 'Car Enthusiasts',            'car-enthusiasts',           true, 280),
    ('group', 'Science & Research',         'science-research',          true, 290),
    ('group', 'Spiritual & Mindfulness',    'spiritual-mindfulness',     true, 300),
    ('group', 'Other',                      'other',                     true, 999)
) AS v(type, name_key, slug, active, sort_order)
WHERE NOT EXISTS (SELECT 1 FROM categories c WHERE c.slug = v.slug AND c.type = v.type);

-- ── Blog categories (25+) ─────────────────────────────────────────────

INSERT INTO categories (type, name_key, slug, active, sort_order)
SELECT v.type, v.name_key, v.slug, v.active, v.sort_order
FROM (VALUES
    ('blog', 'Technology',                  'technology',                true, 10),
    ('blog', 'Lifestyle',                   'lifestyle',                 true, 20),
    ('blog', 'Travel',                      'travel',                    true, 30),
    ('blog', 'Food & Recipes',              'food-recipes',              true, 40),
    ('blog', 'Health & Wellness',           'health-wellness',           true, 50),
    ('blog', 'Business & Entrepreneurship', 'business-entrepreneurship', true, 60),
    ('blog', 'Finance & Investing',         'finance-investing',         true, 70),
    ('blog', 'Fashion & Style',             'fashion-style',             true, 80),
    ('blog', 'Beauty & Skincare',           'beauty-skincare',           true, 90),
    ('blog', 'Sports',                      'sports',                    true, 100),
    ('blog', 'Entertainment & Pop Culture', 'entertainment-pop-culture', true, 110),
    ('blog', 'Science & Research',          'science-research',          true, 120),
    ('blog', 'Education & Learning',        'education-learning',        true, 130),
    ('blog', 'Parenting & Family',          'parenting-family',          true, 140),
    ('blog', 'DIY & Crafts',                'diy-crafts',                true, 150),
    ('blog', 'Gaming',                      'gaming',                    true, 160),
    ('blog', 'Music',                       'music',                     true, 170),
    ('blog', 'Politics & Current Events',   'politics-current-events',   true, 180),
    ('blog', 'Environment & Sustainability', 'environment-sustainability', true, 190),
    ('blog', 'Personal Development',        'personal-development',      true, 200),
    ('blog', 'Career & Professional',       'career-professional',       true, 210),
    ('blog', 'Relationships & Dating',      'relationships-dating',      true, 220),
    ('blog', 'Home & Garden',               'home-garden',               true, 230),
    ('blog', 'Pets & Animals',              'pets-animals',              true, 240),
    ('blog', 'Automotive',                  'automotive',                true, 250),
    ('blog', 'Other',                       'other',                     true, 999)
) AS v(type, name_key, slug, active, sort_order)
WHERE NOT EXISTS (SELECT 1 FROM categories c WHERE c.slug = v.slug AND c.type = v.type);

-- ═══════════════════════════════════════════════════════════════════════
-- Subcategories — 5-10 per major parent category
-- Uses DO block to resolve category IDs by slug + type for idempotency.
-- ═══════════════════════════════════════════════════════════════════════

DO $$
DECLARE
    cat_id BIGINT;
BEGIN
    -- ── Product subcategories ─────────────────────────────────────────

    SELECT id INTO cat_id FROM categories WHERE slug = 'electronics' AND type = 'product';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Smartphones', 'product'),
            ('Laptops', 'product'),
            ('Tablets', 'product'),
            ('Audio & Headphones', 'product'),
            ('Cameras', 'product'),
            ('Wearables', 'product'),
            ('Gaming Consoles', 'product'),
            ('Accessories', 'product'),
            ('Components & Parts', 'product'),
            ('Drones', 'product')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'clothing-fashion' AND type = 'product';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Men''s Clothing', 'product'),
            ('Women''s Clothing', 'product'),
            ('Unisex Clothing', 'product'),
            ('Kids'' Clothing', 'product'),
            ('Accessories', 'product'),
            ('Formal Wear', 'product'),
            ('Casual Wear', 'product'),
            ('Activewear', 'product'),
            ('Traditional Wear', 'product'),
            ('Seasonal & Outerwear', 'product')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'furniture-home' AND type = 'product';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Sofas & Couches', 'product'),
            ('Beds & Mattresses', 'product'),
            ('Tables & Chairs', 'product'),
            ('Storage & Shelving', 'product'),
            ('Desks & Office Furniture', 'product'),
            ('Outdoor Furniture', 'product'),
            ('Lighting', 'product'),
            ('Rugs & Carpets', 'product')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'cars-vehicles' AND type = 'product';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Sedans', 'product'),
            ('SUVs & Crossovers', 'product'),
            ('Trucks', 'product'),
            ('Motorcycles', 'product'),
            ('Electric Vehicles', 'product'),
            ('Classic & Vintage Cars', 'product'),
            ('Parts & Accessories', 'product'),
            ('Boats & Watercraft', 'product')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'books-magazines' AND type = 'product';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Fiction', 'product'),
            ('Non-Fiction', 'product'),
            ('Textbooks & Educational', 'product'),
            ('Children''s Books', 'product'),
            ('Comics & Manga', 'product'),
            ('Magazines', 'product'),
            ('Rare & First Editions', 'product'),
            ('E-Books', 'product')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'sports-outdoors' AND type = 'product';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Team Sports', 'product'),
            ('Running & Jogging', 'product'),
            ('Yoga & Pilates', 'product'),
            ('Cycling', 'product'),
            ('Water Sports', 'product'),
            ('Winter Sports', 'product'),
            ('Camping & Hiking Gear', 'product'),
            ('Fishing Equipment', 'product'),
            ('Hunting Gear', 'product'),
            ('Sportswear', 'product')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'toys-games' AND type = 'product';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Action Figures', 'product'),
            ('Dolls & Accessories', 'product'),
            ('Building Sets', 'product'),
            ('Educational Toys', 'product'),
            ('Remote Control Toys', 'product'),
            ('Stuffed Animals', 'product'),
            ('Outdoor Play', 'product'),
            ('Puzzles', 'product')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'musical-instruments' AND type = 'product';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Guitars', 'product'),
            ('Pianos & Keyboards', 'product'),
            ('Drums & Percussion', 'product'),
            ('Wind Instruments', 'product'),
            ('String Instruments', 'product'),
            ('DJ & Electronic Gear', 'product'),
            ('Studio Equipment', 'product'),
            ('Accessories & Parts', 'product')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'pet-supplies' AND type = 'product';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Dog Supplies', 'product'),
            ('Cat Supplies', 'product'),
            ('Fish & Aquarium', 'product'),
            ('Bird Supplies', 'product'),
            ('Small Pets', 'product'),
            ('Reptile Supplies', 'product'),
            ('Pet Food', 'product'),
            ('Pet Health & Grooming', 'product')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'baby-kids' AND type = 'product';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Baby Clothing', 'product'),
            ('Nursery Furniture', 'product'),
            ('Strollers & Car Seats', 'product'),
            ('Feeding & Nursing', 'product'),
            ('Diapering', 'product'),
            ('Baby Toys', 'product'),
            ('Maternity', 'product'),
            ('Kids'' Room Decor', 'product')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'jewelry-watches' AND type = 'product';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Necklaces & Pendants', 'product'),
            ('Rings', 'product'),
            ('Earrings', 'product'),
            ('Bracelets', 'product'),
            ('Watches', 'product'),
            ('Luxury & Fine Jewelry', 'product'),
            ('Handmade Jewelry', 'product'),
            ('Men''s Jewelry', 'product')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'shoes-footwear' AND type = 'product';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Sneakers', 'product'),
            ('Boots', 'product'),
            ('Sandals & Flip-Flops', 'product'),
            ('Formal Shoes', 'product'),
            ('Heels & Pumps', 'product'),
            ('Athletic & Running', 'product'),
            ('Slippers', 'product'),
            ('Loafers & Flats', 'product')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'health-beauty' AND type = 'product';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Skincare', 'product'),
            ('Makeup', 'product'),
            ('Hair Care', 'product'),
            ('Fragrance', 'product'),
            ('Nail Care', 'product'),
            ('Bath & Body', 'product'),
            ('Men''s Grooming', 'product'),
            ('Wellness & Supplements', 'product')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'kitchen-appliances' AND type = 'product';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Refrigerators', 'product'),
            ('Ovens & Stoves', 'product'),
            ('Microwaves', 'product'),
            ('Blenders & Mixers', 'product'),
            ('Coffee Makers', 'product'),
            ('Cookware & Bakeware', 'product'),
            ('Cutlery & Knives', 'product'),
            ('Food Storage', 'product')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'smartphones-tablets' AND type = 'product';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Android Phones', 'product'),
            ('iPhones', 'product'),
            ('iPads & Tablets', 'product'),
            ('Cases & Covers', 'product'),
            ('Screen Protectors', 'product'),
            ('Chargers & Cables', 'product'),
            ('Power Banks', 'product'),
            ('Phone Mounts & Stands', 'product')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'laptops-computers' AND type = 'product';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Laptops & Notebooks', 'product'),
            ('Desktop PCs', 'product'),
            ('Monitors & Displays', 'product'),
            ('Keyboards & Mice', 'product'),
            ('Storage & Hard Drives', 'product'),
            ('Computer Components', 'product'),
            ('Printers & Scanners', 'product'),
            ('Software & Licenses', 'product')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'home-decor' AND type = 'product';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Wall Art & Prints', 'product'),
            ('Vases & Plants', 'product'),
            ('Mirrors', 'product'),
            ('Curtains & Blinds', 'product'),
            ('Throw Pillows & Blankets', 'product'),
            ('Candles & Scents', 'product'),
            ('Clocks', 'product'),
            ('Picture Frames', 'product')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'video-games-consoles' AND type = 'product';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('PlayStation Games', 'product'),
            ('Xbox Games', 'product'),
            ('Nintendo Games', 'product'),
            ('PC Games', 'product'),
            ('Consoles', 'product'),
            ('Controllers & Accessories', 'product'),
            ('Retro Gaming', 'product'),
            ('Gaming Merchandise', 'product')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'art-crafts' AND type = 'product';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Paintings', 'product'),
            ('Sculptures', 'product'),
            ('Handmade Crafts', 'product'),
            ('Knitting & Sewing', 'product'),
            ('Drawing & Sketching', 'product'),
            ('Art Supplies', 'product'),
            ('Craft Kits', 'product'),
            ('Custom & Personalized', 'product')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    -- ── Job subcategories ──────────────────────────────────────────────

    SELECT id INTO cat_id FROM categories WHERE slug = 'software-engineering' AND type = 'job';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Frontend Development', 'job'),
            ('Backend Development', 'job'),
            ('Full-Stack Development', 'job'),
            ('Mobile Development', 'job'),
            ('DevOps & SRE', 'job'),
            ('Quality Assurance & Testing', 'job'),
            ('Embedded Systems', 'job'),
            ('Game Development', 'job')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'design-creative' AND type = 'job';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('UI/UX Design', 'job'),
            ('Graphic Design', 'job'),
            ('Product Design', 'job'),
            ('Motion Graphics', 'job'),
            ('Illustration', 'job'),
            ('Interior Design', 'job'),
            ('Fashion Design', 'job'),
            ('Video Editing & Production', 'job')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'marketing-advertising' AND type = 'job';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Digital Marketing', 'job'),
            ('Content Marketing', 'job'),
            ('Social Media Marketing', 'job'),
            ('SEO & SEM', 'job'),
            ('Email Marketing', 'job'),
            ('Brand Strategy', 'job'),
            ('Market Research', 'job'),
            ('Public Relations', 'job')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'finance-accounting' AND type = 'job';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Financial Analysis', 'job'),
            ('Accounting & Bookkeeping', 'job'),
            ('Audit & Compliance', 'job'),
            ('Investment Banking', 'job'),
            ('Tax Preparation', 'job'),
            ('Risk Management', 'job'),
            ('Financial Planning', 'job'),
            ('Payroll', 'job')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'healthcare-medical' AND type = 'job';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Nursing', 'job'),
            ('Physician & Doctor', 'job'),
            ('Dentistry', 'job'),
            ('Pharmacy', 'job'),
            ('Physical Therapy', 'job'),
            ('Mental Health & Counseling', 'job'),
            ('Veterinary', 'job'),
            ('Medical Administration', 'job')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'education-teaching' AND type = 'job';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Primary Education', 'job'),
            ('Secondary Education', 'job'),
            ('Higher Education', 'job'),
            ('ESL & Language Teaching', 'job'),
            ('Special Education', 'job'),
            ('Online Teaching & Tutoring', 'job'),
            ('Curriculum Development', 'job'),
            ('Early Childhood Education', 'job')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'data-science-analytics' AND type = 'job';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Data Analysis', 'job'),
            ('Machine Learning Engineering', 'job'),
            ('Data Engineering', 'job'),
            ('Business Intelligence', 'job'),
            ('Statistical Modeling', 'job'),
            ('NLP & Text Analytics', 'job'),
            ('Computer Vision', 'job'),
            ('Research Science', 'job')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'sales-business-dev' AND type = 'job';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('B2B Sales', 'job'),
            ('B2C Sales', 'job'),
            ('Inside Sales', 'job'),
            ('Account Management', 'job'),
            ('Sales Operations', 'job'),
            ('Partnerships', 'job'),
            ('Lead Generation', 'job'),
            ('Enterprise Sales', 'job')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'human-resources' AND type = 'job';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Recruitment & Talent Acquisition', 'job'),
            ('Employee Relations', 'job'),
            ('Compensation & Benefits', 'job'),
            ('Training & Development', 'job'),
            ('HR Operations', 'job'),
            ('Organizational Development', 'job'),
            ('Workplace Safety', 'job'),
            ('Diversity & Inclusion', 'job')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    -- ── Page subcategories ─────────────────────────────────────────────

    SELECT id INTO cat_id FROM categories WHERE slug = 'restaurant' AND type = 'page';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Italian Restaurant', 'page'),
            ('Asian Cuisine', 'page'),
            ('Fast Food', 'page'),
            ('Fine Dining', 'page'),
            ('Vegan & Vegetarian', 'page'),
            ('Seafood', 'page'),
            ('Steakhouse', 'page'),
            ('Buffet & All-You-Can-Eat', 'page')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'cafe-coffee-shop' AND type = 'page';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Specialty Coffee', 'page'),
            ('Tea House', 'page'),
            ('Bakery Cafe', 'page'),
            ('Internet Cafe', 'page'),
            ('Juice & Smoothie Bar', 'page'),
            ('Cat Cafe', 'page'),
            ('Bookstore Cafe', 'page'),
            ('Dessert Cafe', 'page')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'retail-store' AND type = 'page';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Grocery & Supermarket', 'page'),
            ('Convenience Store', 'page'),
            ('Department Store', 'page'),
            ('Thrift & Secondhand', 'page'),
            ('Bookstore', 'page'),
            ('Gift Shop', 'page'),
            ('Specialty Foods', 'page'),
            ('Electronics Store', 'page')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'salon-spa' AND type = 'page';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Hair Salon', 'page'),
            ('Barbershop', 'page'),
            ('Nail Salon', 'page'),
            ('Day Spa', 'page'),
            ('Massage Therapy', 'page'),
            ('Waxing & Hair Removal', 'page'),
            ('Skin Care Clinic', 'page'),
            ('Tattoo & Piercing', 'page')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'gym-fitness-center' AND type = 'page';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('General Gym', 'page'),
            ('CrossFit Box', 'page'),
            ('Yoga Studio', 'page'),
            ('Pilates Studio', 'page'),
            ('Martial Arts Dojo', 'page'),
            ('Boxing Gym', 'page'),
            ('Climbing Gym', 'page'),
            ('Swimming Pool', 'page')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'hotel-accommodation' AND type = 'page';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Hotel', 'page'),
            ('Hostel', 'page'),
            ('Bed & Breakfast', 'page'),
            ('Resort', 'page'),
            ('Vacation Rental', 'page'),
            ('Motel', 'page'),
            ('Boutique Hotel', 'page'),
            ('Guest House', 'page')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'event-planning' AND type = 'page';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Wedding Planning', 'page'),
            ('Corporate Events', 'page'),
            ('Birthday Parties', 'page'),
            ('Catering Services', 'page'),
            ('Venue Rental', 'page'),
            ('Floral & Decor', 'page'),
            ('Entertainment Booking', 'page'),
            ('Festival & Fair', 'page')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'home-services' AND type = 'page';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Plumbing', 'page'),
            ('Electrical', 'page'),
            ('HVAC', 'page'),
            ('Painting', 'page'),
            ('Roofing', 'page'),
            ('Moving & Relocation', 'page'),
            ('Pest Control', 'page'),
            ('Handyman', 'page')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    -- ── Group subcategories ────────────────────────────────────────────

    SELECT id INTO cat_id FROM categories WHERE slug = 'professional-networking' AND type = 'group';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Industry Professionals', 'group'),
            ('Job Seekers & Hiring', 'group'),
            ('Women in Business', 'group'),
            ('Remote Workers', 'group'),
            ('Freelancers & Contractors', 'group'),
            ('Mentorship', 'group'),
            ('Young Professionals', 'group'),
            ('Executive Leaders', 'group')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'gaming-esports' AND type = 'group';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('PC Gaming', 'group'),
            ('Console Gaming', 'group'),
            ('Mobile Gaming', 'group'),
            ('Esports Teams', 'group'),
            ('Tabletop RPG', 'group'),
            ('Retro Gaming', 'group'),
            ('Game Development', 'group'),
            ('MMO Guilds', 'group')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'music-bands' AND type = 'group';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Rock & Metal', 'group'),
            ('Electronic & EDM', 'group'),
            ('Hip-Hop & Rap', 'group'),
            ('Jazz & Blues', 'group'),
            ('Classical & Orchestra', 'group'),
            ('Indie & Alternative', 'group'),
            ('Musicians Seeking Bands', 'group'),
            ('DJ & Production', 'group')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'sports-recreation' AND type = 'group';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Soccer / Football', 'group'),
            ('Basketball', 'group'),
            ('Tennis', 'group'),
            ('Running Club', 'group'),
            ('Swimming', 'group'),
            ('Martial Arts', 'group'),
            ('Volleyball', 'group'),
            ('Rugby', 'group')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'tech-coding' AND type = 'group';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Web Development', 'group'),
            ('Mobile App Development', 'group'),
            ('Python & Data Science', 'group'),
            ('AI & Machine Learning', 'group'),
            ('Cybersecurity', 'group'),
            ('Blockchain & Crypto', 'group'),
            ('Hardware & IoT', 'group'),
            ('Open Source Projects', 'group')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    -- ── Blog subcategories ─────────────────────────────────────────────

    SELECT id INTO cat_id FROM categories WHERE slug = 'technology' AND type = 'blog';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Software & Apps', 'blog'),
            ('Gadgets & Reviews', 'blog'),
            ('AI & Machine Learning', 'blog'),
            ('Cybersecurity', 'blog'),
            ('Web Development', 'blog'),
            ('Mobile Tech', 'blog'),
            ('Startup News', 'blog'),
            ('How-To & Tutorials', 'blog')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'food-recipes' AND type = 'blog';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Baking & Desserts', 'blog'),
            ('Healthy Eating', 'blog'),
            ('Vegan & Plant-Based', 'blog'),
            ('International Cuisine', 'blog'),
            ('Quick & Easy Meals', 'blog'),
            ('Meal Prep', 'blog'),
            ('Restaurant Reviews', 'blog'),
            ('Cooking Techniques', 'blog')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'travel' AND type = 'blog';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Adventure Travel', 'blog'),
            ('Budget Travel', 'blog'),
            ('Luxury Travel', 'blog'),
            ('Solo Travel', 'blog'),
            ('Travel Tips & Guides', 'blog'),
            ('Cultural Experiences', 'blog'),
            ('Digital Nomad Life', 'blog'),
            ('Family Travel', 'blog')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'health-wellness' AND type = 'blog';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Mental Health', 'blog'),
            ('Nutrition & Diet', 'blog'),
            ('Fitness & Exercise', 'blog'),
            ('Sleep & Recovery', 'blog'),
            ('Mindfulness & Meditation', 'blog'),
            ('Women''s Health', 'blog'),
            ('Men''s Health', 'blog'),
            ('Alternative Medicine', 'blog')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

    SELECT id INTO cat_id FROM categories WHERE slug = 'business-entrepreneurship' AND type = 'blog';
    IF FOUND THEN
        INSERT INTO sub_categories (category_id, lang_key, type)
        SELECT cat_id, v.lang, v.typ FROM (VALUES
            ('Startup Stories', 'blog'),
            ('Marketing Strategies', 'blog'),
            ('Leadership & Management', 'blog'),
            ('Productivity & Workflow', 'blog'),
            ('Funding & Investment', 'blog'),
            ('E-Commerce', 'blog'),
            ('Side Hustles', 'blog'),
            ('Business Tools & Resources', 'blog')
        ) AS v(lang, typ)
        WHERE NOT EXISTS (SELECT 1 FROM sub_categories sc WHERE sc.category_id = cat_id AND sc.lang_key = v.lang);
    END IF;

END $$;

-- ── Gift categories (expanded) ────────────────────────────────────────

INSERT INTO gift_categories (name, is_active, sort_order)
SELECT v.name, v.is_active, v.sort_order
FROM (VALUES
    ('Birthday',          true, 10),
    ('Anniversary',       true, 20),
    ('Wedding',           true, 30),
    ('Valentine''s Day',  true, 40),
    ('Christmas',         true, 50),
    ('New Year',          true, 60),
    ('Graduation',        true, 70),
    ('Baby Shower',       true, 80),
    ('Housewarming',      true, 90),
    ('Thank You',         true, 100),
    ('Get Well Soon',     true, 110),
    ('Congratulations',   true, 120),
    ('Just Because',      true, 130),
    ('Friendship',        true, 140),
    ('Mother''s Day',     true, 150),
    ('Father''s Day',     true, 160),
    ('Easter',            true, 170),
    ('Halloween',         true, 180),
    ('Thanksgiving',      true, 190),
    ('Retirement',        true, 200),
    ('Sympathy',          true, 210),
    ('Engagement',        true, 220),
    ('Promotion',         true, 230),
    ('Good Luck',         true, 240),
    ('Apology',           true, 250)
) AS v(name, is_active, sort_order)
WHERE NOT EXISTS (SELECT 1 FROM gift_categories gc WHERE gc.name = v.name);
