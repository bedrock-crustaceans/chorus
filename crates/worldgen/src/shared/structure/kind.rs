#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StructureKind(pub u16);

#[allow(non_upper_case_globals)]
impl StructureKind {
    pub const Village: Self = Self(0);
    pub const Stronghold: Self = Self(1);
    pub const Temple: Self = Self(2);
    pub const Mineshaft: Self = Self(3);
    pub const Mansion: Self = Self(4);
    pub const BuriedTreasure: Self = Self(5);
    pub const Outpost: Self = Self(6);
    pub const Monument: Self = Self(7);
    pub const OceanRuins: Self = Self(8);
    pub const Shipwreck: Self = Self(9);
    pub const RuinedPortal: Self = Self(10);
    pub const AncientCity: Self = Self(11);
    pub const TrialChambers: Self = Self(12);
    pub const TrailRuins: Self = Self(13);
    pub const AbandonedCamp: Self = Self(14);
}

const BUILT_IN_NAMES: [(&str, StructureKind); 15] = [
    ("village", StructureKind::Village),
    ("stronghold", StructureKind::Stronghold),
    ("temple", StructureKind::Temple),
    ("mineshaft", StructureKind::Mineshaft),
    ("mansion", StructureKind::Mansion),
    ("buried_treasure", StructureKind::BuriedTreasure),
    ("outpost", StructureKind::Outpost),
    ("monument", StructureKind::Monument),
    ("ocean_ruins", StructureKind::OceanRuins),
    ("shipwreck", StructureKind::Shipwreck),
    ("ruined_portal", StructureKind::RuinedPortal),
    ("ancient_city", StructureKind::AncientCity),
    ("trial_chambers", StructureKind::TrialChambers),
    ("trail_ruins", StructureKind::TrailRuins),
    ("abandoned_camp", StructureKind::AbandonedCamp),
];

impl StructureKind {
    pub const BUILT_IN_COUNT: u16 = 15;

    pub const ALL: [StructureKind; 15] = [
        Self::Village,
        Self::Stronghold,
        Self::Temple,
        Self::Mineshaft,
        Self::Mansion,
        Self::BuriedTreasure,
        Self::Outpost,
        Self::Monument,
        Self::OceanRuins,
        Self::Shipwreck,
        Self::RuinedPortal,
        Self::AncientCity,
        Self::TrialChambers,
        Self::TrailRuins,
        Self::AbandonedCamp,
    ];

    pub fn from_variant(name: &str) -> Option<Self> {
        BUILT_IN_NAMES.iter().find(|(known, _)| *known == name).map(|&(_, kind)| kind)
    }
}

impl std::fmt::Debug for StructureKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match BUILT_IN_NAMES.iter().find(|(_, kind)| kind == self) {
            Some((name, _)) => formatter.write_str(name),
            None => write!(formatter, "StructureKind({})", self.0),
        }
    }
}
