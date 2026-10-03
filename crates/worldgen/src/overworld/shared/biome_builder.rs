use super::biome::Biome::{self, *};

#[derive(Clone, Copy)]
pub struct Parameter {
    pub min: i64,
    pub max: i64,
}

fn quantize(value: f32) -> i64 {
    (value * 10000.0) as i64
}

fn span(min: f32, max: f32) -> Parameter {
    Parameter {
        min: quantize(min),
        max: quantize(max),
    }
}

fn point(value: f32) -> Parameter {
    span(value, value)
}

fn join(min: Parameter, max: Parameter) -> Parameter {
    Parameter { min: min.min, max: max.max }
}

pub type ParameterPoint = [Parameter; 7];

const MIDDLE_BIOMES: [[Biome; 5]; 5] = [
    [SnowyPlains, SnowyPlains, SnowyPlains, SnowyTaiga, Taiga],
    [Plains, Plains, Forest, Taiga, OldGrowthSpruceTaiga],
    [FlowerForest, Plains, Forest, BirchForest, DarkForest],
    [Savanna, Savanna, Forest, Jungle, Jungle],
    [Desert, Desert, Desert, Desert, Desert],
];

const MIDDLE_BIOMES_VARIANT: [[Option<Biome>; 5]; 5] = [
    [Some(IceSpikes), None, Some(SnowyTaiga), None, None],
    [Some(DappledForest), None, None, None, Some(OldGrowthPineTaiga)],
    [Some(SunflowerPlains), None, None, Some(OldGrowthBirchForest), None],
    [None, None, Some(Plains), Some(SparseJungle), Some(BambooJungle)],
    [None, None, None, None, None],
];

const PLATEAU_BIOMES: [[Biome; 5]; 5] = [
    [SnowyPlains, SnowyPlains, SnowyPlains, SnowyTaiga, SnowyTaiga],
    [Meadow, Meadow, Forest, Taiga, OldGrowthSpruceTaiga],
    [Meadow, Meadow, Meadow, Meadow, PaleGarden],
    [SavannaPlateau, SavannaPlateau, Forest, Forest, Jungle],
    [Badlands, Badlands, Badlands, WoodedBadlands, WoodedBadlands],
];

const PLATEAU_BIOMES_VARIANT: [[Option<Biome>; 5]; 5] = [
    [Some(IceSpikes), None, None, None, None],
    [Some(CherryGrove), None, Some(Meadow), Some(Meadow), Some(OldGrowthPineTaiga)],
    [Some(CherryGrove), Some(CherryGrove), Some(Forest), Some(BirchForest), None],
    [None, None, None, None, None],
    [Some(ErodedBadlands), Some(ErodedBadlands), None, None, None],
];

const SHATTERED_BIOMES: [[Option<Biome>; 5]; 5] = [
    [
        Some(WindsweptGravellyHills),
        Some(WindsweptGravellyHills),
        Some(WindsweptHills),
        Some(WindsweptForest),
        Some(WindsweptForest),
    ],
    [
        Some(WindsweptGravellyHills),
        Some(WindsweptGravellyHills),
        Some(WindsweptHills),
        Some(WindsweptForest),
        Some(WindsweptForest),
    ],
    [Some(WindsweptHills), Some(WindsweptHills), Some(WindsweptHills), Some(WindsweptForest), Some(WindsweptForest)],
    [None, None, None, None, None],
    [None, None, None, None, None],
];

const OCEANS: [[Biome; 5]; 2] = [
    [DeepFrozenOcean, DeepColdOcean, DeepOcean, DeepLukewarmOcean, WarmOcean],
    [FrozenOcean, ColdOcean, Ocean, LukewarmOcean, WarmOcean],
];

pub struct OverworldBiomeBuilder {
    full: Parameter,
    temperatures: [Parameter; 5],
    humidities: [Parameter; 5],
    erosions: [Parameter; 7],
    frozen: Parameter,
    unfrozen: Parameter,
    mushroom_fields: Parameter,
    deep_ocean: Parameter,
    ocean: Parameter,
    coast: Parameter,
    inland: Parameter,
    near_inland: Parameter,
    mid_inland: Parameter,
    far_inland: Parameter,
    out: Vec<(ParameterPoint, Biome)>,
}

impl OverworldBiomeBuilder {
    pub fn build() -> Vec<(ParameterPoint, Biome)> {
        let temperatures = [span(-1.0, -0.45), span(-0.45, -0.15), span(-0.15, 0.2), span(0.2, 0.55), span(0.55, 1.0)];
        let mut builder = Self {
            full: span(-1.0, 1.0),
            temperatures,
            humidities: [span(-1.0, -0.35), span(-0.35, -0.1), span(-0.1, 0.1), span(0.1, 0.3), span(0.3, 1.0)],
            erosions: [
                span(-1.0, -0.78),
                span(-0.78, -0.375),
                span(-0.375, -0.2225),
                span(-0.2225, 0.05),
                span(0.05, 0.45),
                span(0.45, 0.55),
                span(0.55, 1.0),
            ],
            frozen: temperatures[0],
            unfrozen: join(temperatures[1], temperatures[4]),
            mushroom_fields: span(-1.2, -1.05),
            deep_ocean: span(-1.05, -0.455),
            ocean: span(-0.455, -0.19),
            coast: span(-0.19, -0.11),
            inland: span(-0.11, 0.55),
            near_inland: span(-0.11, 0.03),
            mid_inland: span(0.03, 0.3),
            far_inland: span(0.3, 1.0),
            out: Vec::new(),
        };
        builder.add_off_coast_biomes();
        builder.add_inland_biomes();
        builder.add_underground_biomes();
        builder.out
    }

    #[allow(clippy::too_many_arguments)]
    fn surface(&mut self, temperature: Parameter, humidity: Parameter, continentalness: Parameter, erosion: Parameter, weirdness: Parameter, biome: Biome) {
        let offset = Parameter { min: 0, max: 0 };
        self.out.push(([temperature, humidity, continentalness, erosion, point(0.0), weirdness, offset], biome));
        self.out.push(([temperature, humidity, continentalness, erosion, point(1.0), weirdness, offset], biome));
    }

    #[allow(clippy::too_many_arguments)]
    fn underground(&mut self, temperature: Parameter, humidity: Parameter, continentalness: Parameter, erosion: Parameter, weirdness: Parameter, biome: Biome) {
        self.out
            .push(([temperature, humidity, continentalness, erosion, span(0.2, 0.9), weirdness, Parameter { min: 0, max: 0 }], biome));
    }

    fn add_off_coast_biomes(&mut self) {
        let full = self.full;
        self.surface(full, full, self.mushroom_fields, full, full, MushroomFields);
        for (t, temperature) in self.temperatures.into_iter().enumerate() {
            self.surface(temperature, full, self.deep_ocean, full, full, OCEANS[0][t]);
            self.surface(temperature, full, self.ocean, full, full, OCEANS[1][t]);
        }
    }

    fn add_inland_biomes(&mut self) {
        self.add_mid_slice(span(-1.0, -0.93333334));
        self.add_high_slice(span(-0.93333334, -0.7666667));
        self.add_peaks(span(-0.7666667, -0.56666666));
        self.add_high_slice(span(-0.56666666, -0.4));
        self.add_mid_slice(span(-0.4, -0.26666668));
        self.add_low_slice(span(-0.26666668, -0.05));
        self.add_valleys(span(-0.05, 0.05));
        self.add_low_slice(span(0.05, 0.26666668));
        self.add_mid_slice(span(0.26666668, 0.4));
        self.add_high_slice(span(0.4, 0.56666666));
        self.add_peaks(span(0.56666666, 0.7666667));
        self.add_high_slice(span(0.7666667, 0.93333334));
        self.add_mid_slice(span(0.93333334, 1.0));
    }

    fn add_peaks(&mut self, weirdness: Parameter) {
        let e = self.erosions;
        for t in 0..5 {
            for h in 0..5 {
                let (temperature, humidity) = (self.temperatures[t], self.humidities[h]);
                let middle = pick_middle(t, h, weirdness);
                let middle_or_badlands = pick_middle_or_badlands_if_hot(t, h, weirdness);
                let middle_or_badlands_or_slope = pick_middle_or_badlands_if_hot_or_slope_if_cold(t, h, weirdness);
                let plateau = pick_plateau(t, h, weirdness);
                let shattered = pick_shattered(t, h, weirdness);
                let shattered_or_savanna = maybe_windswept_savanna(t, h, weirdness, shattered);
                let peak = pick_peak(t, h, weirdness);
                self.surface(temperature, humidity, join(self.coast, self.far_inland), e[0], weirdness, peak);
                self.surface(temperature, humidity, join(self.coast, self.near_inland), e[1], weirdness, middle_or_badlands_or_slope);
                self.surface(temperature, humidity, join(self.mid_inland, self.far_inland), e[1], weirdness, peak);
                self.surface(temperature, humidity, join(self.coast, self.near_inland), join(e[2], e[3]), weirdness, middle);
                self.surface(temperature, humidity, join(self.mid_inland, self.far_inland), e[2], weirdness, plateau);
                self.surface(temperature, humidity, self.mid_inland, e[3], weirdness, middle_or_badlands);
                self.surface(temperature, humidity, self.far_inland, e[3], weirdness, plateau);
                self.surface(temperature, humidity, join(self.coast, self.far_inland), e[4], weirdness, middle);
                self.surface(temperature, humidity, join(self.coast, self.near_inland), e[5], weirdness, shattered_or_savanna);
                self.surface(temperature, humidity, join(self.mid_inland, self.far_inland), e[5], weirdness, shattered);
                self.surface(temperature, humidity, join(self.coast, self.far_inland), e[6], weirdness, middle);
            }
        }
    }

    fn add_high_slice(&mut self, weirdness: Parameter) {
        let e = self.erosions;
        for t in 0..5 {
            for h in 0..5 {
                let (temperature, humidity) = (self.temperatures[t], self.humidities[h]);
                let middle = pick_middle(t, h, weirdness);
                let middle_or_badlands = pick_middle_or_badlands_if_hot(t, h, weirdness);
                let middle_or_badlands_or_slope = pick_middle_or_badlands_if_hot_or_slope_if_cold(t, h, weirdness);
                let plateau = pick_plateau(t, h, weirdness);
                let shattered = pick_shattered(t, h, weirdness);
                let middle_or_savanna = maybe_windswept_savanna(t, h, weirdness, middle);
                let slope = pick_slope(t, h, weirdness);
                let peak = pick_peak(t, h, weirdness);
                self.surface(temperature, humidity, self.coast, join(e[0], e[1]), weirdness, middle);
                self.surface(temperature, humidity, self.near_inland, e[0], weirdness, slope);
                self.surface(temperature, humidity, join(self.mid_inland, self.far_inland), e[0], weirdness, peak);
                self.surface(temperature, humidity, self.near_inland, e[1], weirdness, middle_or_badlands_or_slope);
                self.surface(temperature, humidity, join(self.mid_inland, self.far_inland), e[1], weirdness, slope);
                self.surface(temperature, humidity, join(self.coast, self.near_inland), join(e[2], e[3]), weirdness, middle);
                self.surface(temperature, humidity, join(self.mid_inland, self.far_inland), e[2], weirdness, plateau);
                self.surface(temperature, humidity, self.mid_inland, e[3], weirdness, middle_or_badlands);
                self.surface(temperature, humidity, self.far_inland, e[3], weirdness, plateau);
                self.surface(temperature, humidity, join(self.coast, self.far_inland), e[4], weirdness, middle);
                self.surface(temperature, humidity, join(self.coast, self.near_inland), e[5], weirdness, middle_or_savanna);
                self.surface(temperature, humidity, join(self.mid_inland, self.far_inland), e[5], weirdness, shattered);
                self.surface(temperature, humidity, join(self.coast, self.far_inland), e[6], weirdness, middle);
            }
        }
    }

    fn add_swamps(&mut self, weirdness: Parameter, continentalness: Parameter) {
        let (t, full, e) = (self.temperatures, self.full, self.erosions);
        self.surface(join(t[1], t[2]), full, continentalness, e[6], weirdness, Swamp);
        self.surface(join(t[3], t[4]), full, continentalness, e[6], weirdness, MangroveSwamp);
    }

    fn add_mid_slice(&mut self, weirdness: Parameter) {
        let e = self.erosions;
        let full = self.full;
        self.surface(full, full, self.coast, join(e[0], e[2]), weirdness, StonyShore);
        self.add_swamps(weirdness, join(self.near_inland, self.far_inland));
        for t in 0..5 {
            for h in 0..5 {
                let (temperature, humidity) = (self.temperatures[t], self.humidities[h]);
                let middle = pick_middle(t, h, weirdness);
                let middle_or_badlands = pick_middle_or_badlands_if_hot(t, h, weirdness);
                let middle_or_badlands_or_slope = pick_middle_or_badlands_if_hot_or_slope_if_cold(t, h, weirdness);
                let shattered = pick_shattered(t, h, weirdness);
                let plateau = pick_plateau(t, h, weirdness);
                let beach = pick_beach(t);
                let middle_or_savanna = maybe_windswept_savanna(t, h, weirdness, middle);
                let shattered_coast = pick_shattered_coast(t, h, weirdness);
                let slope = pick_slope(t, h, weirdness);
                self.surface(temperature, humidity, join(self.near_inland, self.far_inland), e[0], weirdness, slope);
                self.surface(temperature, humidity, join(self.near_inland, self.mid_inland), e[1], weirdness, middle_or_badlands_or_slope);
                self.surface(temperature, humidity, self.far_inland, e[1], weirdness, if t == 0 { slope } else { plateau });
                self.surface(temperature, humidity, self.near_inland, e[2], weirdness, middle);
                self.surface(temperature, humidity, self.mid_inland, e[2], weirdness, middle_or_badlands);
                self.surface(temperature, humidity, self.far_inland, e[2], weirdness, plateau);
                self.surface(temperature, humidity, join(self.coast, self.near_inland), e[3], weirdness, middle);
                self.surface(temperature, humidity, join(self.mid_inland, self.far_inland), e[3], weirdness, middle_or_badlands);
                if weirdness.max < 0 {
                    self.surface(temperature, humidity, self.coast, e[4], weirdness, beach);
                    self.surface(temperature, humidity, join(self.near_inland, self.far_inland), e[4], weirdness, middle);
                } else {
                    self.surface(temperature, humidity, join(self.coast, self.far_inland), e[4], weirdness, middle);
                }
                self.surface(temperature, humidity, self.coast, e[5], weirdness, shattered_coast);
                self.surface(temperature, humidity, self.near_inland, e[5], weirdness, middle_or_savanna);
                self.surface(temperature, humidity, join(self.mid_inland, self.far_inland), e[5], weirdness, shattered);
                self.surface(temperature, humidity, self.coast, e[6], weirdness, if weirdness.max < 0 { beach } else { middle });
                if t == 0 {
                    self.surface(temperature, humidity, join(self.near_inland, self.far_inland), e[6], weirdness, middle);
                }
            }
        }
    }

    fn add_low_slice(&mut self, weirdness: Parameter) {
        let e = self.erosions;
        let full = self.full;
        self.surface(full, full, self.coast, join(e[0], e[2]), weirdness, StonyShore);
        self.add_swamps(weirdness, join(self.near_inland, self.far_inland));
        for t in 0..5 {
            for h in 0..5 {
                let (temperature, humidity) = (self.temperatures[t], self.humidities[h]);
                let middle = pick_middle(t, h, weirdness);
                let middle_or_badlands = pick_middle_or_badlands_if_hot(t, h, weirdness);
                let middle_or_badlands_or_slope = pick_middle_or_badlands_if_hot_or_slope_if_cold(t, h, weirdness);
                let beach = pick_beach(t);
                let middle_or_savanna = maybe_windswept_savanna(t, h, weirdness, middle);
                let shattered_coast = pick_shattered_coast(t, h, weirdness);
                self.surface(temperature, humidity, self.near_inland, join(e[0], e[1]), weirdness, middle_or_badlands);
                self.surface(temperature, humidity, join(self.mid_inland, self.far_inland), join(e[0], e[1]), weirdness, middle_or_badlands_or_slope);
                self.surface(temperature, humidity, self.near_inland, join(e[2], e[3]), weirdness, middle);
                self.surface(temperature, humidity, join(self.mid_inland, self.far_inland), join(e[2], e[3]), weirdness, middle_or_badlands);
                self.surface(temperature, humidity, self.coast, join(e[3], e[4]), weirdness, beach);
                self.surface(temperature, humidity, join(self.near_inland, self.far_inland), e[4], weirdness, middle);
                self.surface(temperature, humidity, self.coast, e[5], weirdness, shattered_coast);
                self.surface(temperature, humidity, self.near_inland, e[5], weirdness, middle_or_savanna);
                self.surface(temperature, humidity, join(self.mid_inland, self.far_inland), e[5], weirdness, middle);
                self.surface(temperature, humidity, self.coast, e[6], weirdness, beach);
                if t == 0 {
                    self.surface(temperature, humidity, join(self.near_inland, self.far_inland), e[6], weirdness, middle);
                }
            }
        }
    }

    fn add_valleys(&mut self, weirdness: Parameter) {
        let e = self.erosions;
        let (full, frozen, unfrozen) = (self.full, self.frozen, self.unfrozen);
        let negative = weirdness.max < 0;
        self.surface(frozen, full, self.coast, join(e[0], e[1]), weirdness, if negative { StonyShore } else { FrozenRiver });
        self.surface(unfrozen, full, self.coast, join(e[0], e[1]), weirdness, if negative { StonyShore } else { River });
        self.surface(frozen, full, self.near_inland, join(e[0], e[1]), weirdness, FrozenRiver);
        self.surface(unfrozen, full, self.near_inland, join(e[0], e[1]), weirdness, River);
        self.surface(frozen, full, join(self.coast, self.far_inland), join(e[2], e[5]), weirdness, FrozenRiver);
        self.surface(unfrozen, full, join(self.coast, self.far_inland), join(e[2], e[5]), weirdness, River);
        self.surface(frozen, full, self.coast, e[6], weirdness, FrozenRiver);
        self.surface(unfrozen, full, self.coast, e[6], weirdness, River);
        self.add_swamps(weirdness, join(self.inland, self.far_inland));
        self.surface(frozen, full, join(self.inland, self.far_inland), e[6], weirdness, FrozenRiver);
        for t in 0..5 {
            for h in 0..5 {
                let biome = pick_middle_or_badlands_if_hot(t, h, weirdness);
                self.surface(self.temperatures[t], self.humidities[h], join(self.mid_inland, self.far_inland), join(e[0], e[1]), weirdness, biome);
            }
        }
    }

    fn add_underground_biomes(&mut self) {
        let (full, e) = (self.full, self.erosions);
        self.underground(full, full, span(0.8, 1.0), full, full, DripstoneCaves);
        self.underground(full, span(0.7, 1.0), full, full, full, LushCaves);
        self.underground(full, full, join(self.coast, self.inland), join(e[5], e[6]), span(-1.1, -0.85), SulfurCaves);
        self.out.push(([full, full, full, join(e[0], e[1]), point(1.1), full, Parameter { min: 0, max: 0 }], DeepDark));
    }
}

fn pick_middle(t: usize, h: usize, weirdness: Parameter) -> Biome {
    if weirdness.max < 0 {
        return MIDDLE_BIOMES[t][h];
    }
    MIDDLE_BIOMES_VARIANT[t][h].unwrap_or(MIDDLE_BIOMES[t][h])
}

fn pick_middle_or_badlands_if_hot(t: usize, h: usize, weirdness: Parameter) -> Biome {
    if t == 4 { pick_badlands(h, weirdness) } else { pick_middle(t, h, weirdness) }
}

fn pick_middle_or_badlands_if_hot_or_slope_if_cold(t: usize, h: usize, weirdness: Parameter) -> Biome {
    if t == 0 { pick_slope(t, h, weirdness) } else { pick_middle_or_badlands_if_hot(t, h, weirdness) }
}

fn maybe_windswept_savanna(t: usize, h: usize, weirdness: Parameter, underlying: Biome) -> Biome {
    if t > 1 && h < 4 && weirdness.max >= 0 { WindsweptSavanna } else { underlying }
}

fn pick_shattered_coast(t: usize, h: usize, weirdness: Parameter) -> Biome {
    let beach_or_middle = if weirdness.max >= 0 { pick_middle(t, h, weirdness) } else { pick_beach(t) };
    maybe_windswept_savanna(t, h, weirdness, beach_or_middle)
}

fn pick_beach(t: usize) -> Biome {
    match t {
        0 => SnowyBeach,
        4 => Desert,
        _ => Beach,
    }
}

fn pick_badlands(h: usize, weirdness: Parameter) -> Biome {
    if h < 2 {
        if weirdness.max < 0 { Badlands } else { ErodedBadlands }
    } else if h < 3 {
        Badlands
    } else {
        WoodedBadlands
    }
}

fn pick_plateau(t: usize, h: usize, weirdness: Parameter) -> Biome {
    if weirdness.max >= 0
        && let Some(variant) = PLATEAU_BIOMES_VARIANT[t][h]
    {
        return variant;
    }
    PLATEAU_BIOMES[t][h]
}

fn pick_peak(t: usize, h: usize, weirdness: Parameter) -> Biome {
    if t <= 2 {
        if weirdness.max < 0 { JaggedPeaks } else { FrozenPeaks }
    } else if t == 3 {
        StonyPeaks
    } else {
        pick_badlands(h, weirdness)
    }
}

fn pick_slope(t: usize, h: usize, weirdness: Parameter) -> Biome {
    if t >= 3 {
        pick_plateau(t, h, weirdness)
    } else if h <= 1 {
        SnowySlopes
    } else {
        Grove
    }
}

fn pick_shattered(t: usize, h: usize, weirdness: Parameter) -> Biome {
    SHATTERED_BIOMES[t][h].unwrap_or_else(|| pick_middle(t, h, weirdness))
}
