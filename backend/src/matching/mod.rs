pub mod bayes;

pub use bayes::{
    GuestStarObs, SupernovaRemnant, MatchCandidate,
    MatchConfig, run_bayesian_match,
};
