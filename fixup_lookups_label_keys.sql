-- Fixup: Update lookups label_keys to be namespace-relative
-- Run this if the migration 20260505000001 has already been applied
-- with the old full-path label_keys.

UPDATE lookups SET label_key = 'fullTime'      WHERE value = 'full_time'     AND lookup_type = 'job_type';
UPDATE lookups SET label_key = 'partTime'      WHERE value = 'part_time'     AND lookup_type = 'job_type';
UPDATE lookups SET label_key = 'contract'       WHERE value = 'contract'       AND lookup_type = 'job_type';
UPDATE lookups SET label_key = 'freelance'      WHERE value = 'freelance'      AND lookup_type = 'job_type';
UPDATE lookups SET label_key = 'internship'     WHERE value = 'internship'     AND lookup_type = 'job_type';
UPDATE lookups SET label_key = 'volunteer'      WHERE value = 'volunteer'      AND lookup_type = 'job_type';
UPDATE lookups SET label_key = 'temporary'      WHERE value = 'temporary'      AND lookup_type = 'job_type';
UPDATE lookups SET label_key = 'remote'         WHERE value = 'remote'         AND lookup_type = 'job_type';

UPDATE lookups SET label_key = 'entry'          WHERE value = 'entry'          AND lookup_type = 'experience';
UPDATE lookups SET label_key = 'mid'            WHERE value = 'mid'            AND lookup_type = 'experience';
UPDATE lookups SET label_key = 'senior'         WHERE value = 'senior'         AND lookup_type = 'experience';
UPDATE lookups SET label_key = 'lead'           WHERE value = 'lead'           AND lookup_type = 'experience';
UPDATE lookups SET label_key = 'executive'      WHERE value = 'executive'      AND lookup_type = 'experience';

UPDATE lookups SET label_key = 'hourly'         WHERE value = 'hourly'         AND lookup_type = 'salary_period';
UPDATE lookups SET label_key = 'daily'          WHERE value = 'daily'          AND lookup_type = 'salary_period';
UPDATE lookups SET label_key = 'weekly'         WHERE value = 'weekly'         AND lookup_type = 'salary_period';
UPDATE lookups SET label_key = 'monthly'        WHERE value = 'monthly'        AND lookup_type = 'salary_period';
UPDATE lookups SET label_key = 'yearly'         WHERE value = 'yearly'         AND lookup_type = 'salary_period';

UPDATE lookups SET label_key = 'benefits.healthInsurance'        WHERE value = 'health_insurance'        AND lookup_type = 'benefit';
UPDATE lookups SET label_key = 'benefits.dentalInsurance'        WHERE value = 'dental_insurance'        AND lookup_type = 'benefit';
UPDATE lookups SET label_key = 'benefits.visionInsurance'        WHERE value = 'vision_insurance'        AND lookup_type = 'benefit';
UPDATE lookups SET label_key = 'benefits.401k'                   WHERE value = 'retirement_401k'         AND lookup_type = 'benefit';
UPDATE lookups SET label_key = 'benefits.stockOptions'           WHERE value = 'stock_options'           AND lookup_type = 'benefit';
UPDATE lookups SET label_key = 'benefits.remoteWork'             WHERE value = 'remote_work'             AND lookup_type = 'benefit';
UPDATE lookups SET label_key = 'benefits.flexibleHours'          WHERE value = 'flexible_hours'          AND lookup_type = 'benefit';
UPDATE lookups SET label_key = 'benefits.paidTimeOff'            WHERE value = 'paid_time_off'           AND lookup_type = 'benefit';
UPDATE lookups SET label_key = 'benefits.parentalLeave'          WHERE value = 'parental_leave'          AND lookup_type = 'benefit';
UPDATE lookups SET label_key = 'benefits.professionalDevelopment' WHERE value = 'professional_development' AND lookup_type = 'benefit';
UPDATE lookups SET label_key = 'benefits.gymMembership'          WHERE value = 'gym_membership'          AND lookup_type = 'benefit';
UPDATE lookups SET label_key = 'benefits.commuterBenefits'       WHERE value = 'commuter_benefits'       AND lookup_type = 'benefit';
UPDATE lookups SET label_key = 'benefits.lifeInsurance'          WHERE value = 'life_insurance'          AND lookup_type = 'benefit';
UPDATE lookups SET label_key = 'benefits.tuitionReimbursement'   WHERE value = 'tuition_reimbursement'   AND lookup_type = 'benefit';
UPDATE lookups SET label_key = 'benefits.employeeDiscount'       WHERE value = 'employee_discount'       AND lookup_type = 'benefit';
UPDATE lookups SET label_key = 'benefits.cateredLunches'         WHERE value = 'catered_lunches'         AND lookup_type = 'benefit';
UPDATE lookups SET label_key = 'benefits.equipmentStipend'       WHERE value = 'equipment_stipend'       AND lookup_type = 'benefit';

UPDATE lookups SET label_key = 'conditionNew'        WHERE value = 'new'        AND lookup_type = 'condition';
UPDATE lookups SET label_key = 'conditionLikeNew'    WHERE value = 'like_new'   AND lookup_type = 'condition';
UPDATE lookups SET label_key = 'conditionGood'       WHERE value = 'good'       AND lookup_type = 'condition';
UPDATE lookups SET label_key = 'conditionFair'       WHERE value = 'fair'       AND lookup_type = 'condition';
UPDATE lookups SET label_key = 'conditionUsed'       WHERE value = 'used'       AND lookup_type = 'condition';
UPDATE lookups SET label_key = 'conditionRefurbished' WHERE value = 'refurbished' AND lookup_type = 'condition';