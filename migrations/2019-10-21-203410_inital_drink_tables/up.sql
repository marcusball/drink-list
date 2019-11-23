-- Your SQL goes here

CREATE TABLE person (
    id SERIAL PRIMARY KEY,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TYPE REALAPPROX AS (val REAL, is_approximate BOOLEAN);

CREATE TABLE drink (
    id              SERIAL PRIMARY KEY,
    name            VARCHAR(128) NOT NULL,
    min_abv         REALAPPROX       NULL, 
    max_abv         REALAPPROX       NULL,
    multiplier      REAL         NOT NULL DEFAULT 1.0,
    created_at      TIMESTAMPTZ  NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at      TIMESTAMPTZ  NOT NULL DEFAULT CURRENT_TIMESTAMP,

    UNIQUE (name, min_abv, max_abv)
);

CREATE INDEX drink_name_lower_idx ON drink (LOWER(name));

COMMENT ON TABLE drink IS 'All drinks, identified by name and alcohol content.';
COMMENT ON COLUMN drink.multiplier IS 'Used when estimating volume content, especially if no ABV is known; Ex: allows for considering 1 double as approximately two units of alcohol.';


CREATE TYPE TIMEPERIOD AS ENUM ('morning', 'afternoon', 'evening', 'night');
COMMENT ON TYPE TIMEPERIOD IS 'The day into vague quarters.';

CREATE TYPE VOLUMEUNIT AS ENUM ('fl oz', 'mL', 'cL', 'L');
COMMENT ON TYPE VOLUMEUNIT IS 'The recognized units of liquid volume measurement.';

CREATE TYPE VOLUME AS (volume REALAPPROX, unit VOLUMEUNIT);

CREATE TABLE entry (
    id                 SERIAL PRIMARY KEY,
    person_id          INTEGER     NOT NULL REFERENCES person(id) ON DELETE CASCADE ON UPDATE CASCADE,
    drank_on           DATE        NOT NULL,
    time_period        TIMEPERIOD  NOT NULL,
    context            TEXT[]      NOT NULL,
    drink_id           INTEGER     NOT NULL REFERENCES drink(id)       ON DELETE NO ACTION ON UPDATE CASCADE,
    min_quantity       REALAPPROX  NOT NULL,
    max_quantity       REALAPPROX  NOT NULL,
    volume             VOLUME          NULL,
    volume_ml          VOLUME          NULL,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX ON entry (person_id, drink_id, drank_on);

COMMENT ON COLUMN entry.time_period IS 'The approximate time of day during which this was drank.';
COMMENT ON COLUMN entry.volume      IS 'The liquid volume of one `quantity` unit of this drink.';
COMMENT ON COLUMN entry.volume_ml   IS 'The `volume` of the drink entry expressed in milliliters.';
COMMENT ON COLUMN entry.context     IS 'Any notes about this entry that add additional context.';

-- Let Diesel manage 'updated_at' columns
SELECT diesel_manage_updated_at('person');
SELECT diesel_manage_updated_at('drink');
SELECT diesel_manage_updated_at('entry');