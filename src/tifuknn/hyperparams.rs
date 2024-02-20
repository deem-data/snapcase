use crate::tifuknn::types::HyperParams;

pub const PARAMS_VALUEDSHOPPER: HyperParams = HyperParams {
    group_size: 7,
    r_basket: 1.0,
    r_group: 0.6,
};

pub const PARAMS_INSTACART: HyperParams = HyperParams {
    group_size: 3,
    r_basket: 0.9,
    r_group: 0.7,
};


pub const PARAMS_TAFANG: HyperParams = HyperParams {
    group_size: 7,
    r_basket: 0.9,
    r_group: 0.7,
};