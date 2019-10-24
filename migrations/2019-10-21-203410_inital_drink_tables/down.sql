-- This file should undo anything in `up.sql`

DROP TABLE IF EXISTS entry;
DROP TABLE IF EXISTS volume_unit;
DROP TABLE IF EXISTS time_period;
DROP TABLE IF EXISTS drink;
DROP TABLE IF EXISTS person;

DROP TYPE IF EXISTS REALAPPROX;