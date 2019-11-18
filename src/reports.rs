use crate::db::Entry;
use crate::models::LiquidVolume;

#[derive(Serialize)]
pub struct DrinkAggregate {
    pub min_drinks: f32,
    pub max_drinks: f32,

    pub min_volume: Option<LiquidVolume>,
    pub max_volume: Option<LiquidVolume>,
}

pub trait DrinkAggregator {
    fn aggregate(&self) -> DrinkAggregate;
}

impl DrinkAggregator for Entry {
    fn aggregate(&self) -> DrinkAggregate {
        // If there is no ABV information, then we'll just assume
        // that each "unit" is 1 drink (times the multiplier).
        if !self.has_abv() || !self.has_volume() {
            return DrinkAggregate {
                min_drinks: self.min_quantity() * self.multiplier,
                max_drinks: self.max_quantity() * self.multiplier,
                min_volume: self.volume.map(|v| {
                    let mut vol = v.clone();
                    vol.amount.num = vol.amount.num * self.min_quantity() * self.multiplier;
                    vol
                }),
                max_volume: self.volume.map(|v| {
                    let mut vol = v.clone();
                    vol.amount.num = vol.amount.num * self.max_quantity() * self.multiplier;
                    vol
                }),
            };
        }

        let min_abv = self.min_abv().expect("Missing min ABV value!");
        let max_abv = self.max_abv().expect("Missing max ABV value!");
        let volume_ml = self.volume_ml.expect("Missing volume!");

        // How many mL of alcohol constitute 1 drink.
        let ml_per_drink = 18.0;

        DrinkAggregate {
            min_drinks: self.min_quantity() * (min_abv / 100.0) * volume_ml.amount.min()
                / ml_per_drink,
            max_drinks: self.max_quantity() * (max_abv / 100.0) * volume_ml.amount.max()
                / ml_per_drink,
            min_volume: self.volume.map(|v| {
                let mut vol = v.clone();
                vol.amount.num = vol.amount.min() * self.min_quantity() * self.multiplier;
                vol
            }),
            max_volume: self.volume.map(|v| {
                let mut vol = v.clone();
                vol.amount.num = vol.amount.max() * self.max_quantity() * self.multiplier;
                vol
            }),
        }
    }
}
