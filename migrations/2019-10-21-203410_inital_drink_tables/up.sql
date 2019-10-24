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

    UNIQUE (name, min_abv, max_abv, multiplier)
);

COMMENT ON TABLE drink IS 'All drinks, identified by name and alcohol content.';
COMMENT ON COLUMN drink.multiplier IS 'Used when estimating volume content, especially if no ABV is known; Ex: allows for considering 1 double as approximately two units of alcohol.';

CREATE TABLE time_period (
    id SERIAL PRIMARY KEY,
    name VARCHAR(16) NOT NULL,

    UNIQUE(name)
);

COMMENT ON TABLE time_period IS 'The day into vaque quarters.';

INSERT INTO time_period (id, name)
VALUES
    (1, 'morning'),
    (2, 'afternoon'), 
    (3, 'evening'),
    (4, 'night');

CREATE TABLE volume_unit (
    id SERIAL PRIMARY KEY,
    abbr VARCHAR(8)  NOT NULL,

    UNIQUE (abbr)
);

INSERT INTO volume_unit (abbr)
VALUES
    ('fl oz'),
    ('ml'),
    ('cl'),
    ('l');

COMMENT ON TABLE volume_unit IS 'The recognized units of liquid volume measurement.';

CREATE TABLE entry (
    id                 SERIAL PRIMARY KEY,
    person_id          INTEGER     NOT NULL REFERENCES person(id) ON DELETE CASCADE ON UPDATE CASCADE,
    drank_on           DATE        NOT NULL,
    time_id            INTEGER     NOT NULL REFERENCES time_period(id) ON DELETE NO ACTION ON UPDATE CASCADE,
    drink_id           INTEGER     NOT NULL REFERENCES drink(id)       ON DELETE NO ACTION ON UPDATE CASCADE,
    min_quantity       REALAPPROX  NOT NULL,
    max_quantity       REALAPPROX  NOT NULL,
    volume             REALAPPROX      NULL,
    volume_unit_id     INTEGER         NULL REFERENCES volume_unit(id) ON DELETE NO ACTION ON UPDATE CASCADE,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX ON entry (person_id, drink_id, drank_on);

-- Let Diesel manage 'updated_at' columns
SELECT diesel_manage_updated_at('person');
SELECT diesel_manage_updated_at('drink');
SELECT diesel_manage_updated_at('entry');