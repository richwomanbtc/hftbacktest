use std::{
    any::Any,
    collections::{hash_map::Entry, HashMap},
    marker::PhantomData,
};

use crate::{
    backtest::BacktestError,
    depth::MarketDepth,
    types::{AnyClone, Order, Side},
};

/// Provides an estimation of the order's queue position.
pub trait QueueModel<MD>
where
    MD: MarketDepth,
{
    /// Initialize the queue position and other necessary values for estimation.
    /// This function is called when the exchange model accepts the new order.
    fn new_order(&self, order: &mut Order, depth: &MD);

    /// Adjusts the estimation values when market trades occur at the same price.
    fn trade(&self, order: &mut Order, qty: f32, depth: &MD);

    /// Adjusts the estimation values when market depth changes at the same price.
    fn depth(&self, order: &mut Order, prev_qty: f32, new_qty: f32, depth: &MD);

    fn is_filled(&self, order: &Order, depth: &MD) -> f32;
}

/// Provides a conservative queue position model, where your order's queue position advances only
/// when trades occur at the same price level.
pub struct RiskAdverseQueueModel<MD>(PhantomData<MD>);

impl AnyClone for f32 {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl<MD> RiskAdverseQueueModel<MD> {
    pub fn new() -> Self {
        Self(Default::default())
    }
}

impl<MD> QueueModel<MD> for RiskAdverseQueueModel<MD>
where
    MD: MarketDepth,
{
    fn new_order(&self, order: &mut Order, depth: &MD) {
        let front_q_qty: f32;
        if order.side == Side::Buy {
            front_q_qty = depth.bid_qty_at_tick(order.price_tick);
        } else {
            front_q_qty = depth.ask_qty_at_tick(order.price_tick);
        }
        order.q = Box::new(front_q_qty);
    }

    fn trade(&self, order: &mut Order, qty: f32, _depth: &MD) {
        let front_q_qty = order.q.as_any_mut().downcast_mut::<f32>().unwrap();
        *front_q_qty -= qty;
    }

    fn depth(&self, order: &mut Order, _prev_qty: f32, new_qty: f32, _depth: &MD) {
        let front_q_qty = order.q.as_any_mut().downcast_mut::<f32>().unwrap();
        *front_q_qty = front_q_qty.min(new_qty);
    }

    fn is_filled(&self, order: &Order, depth: &MD) -> f32 {
        let front_q_qty = order.q.as_any().downcast_ref::<f32>().unwrap();
        if (front_q_qty / depth.lot_size()).round() < 0.0 {
            let q_qty = (-front_q_qty / depth.lot_size()).floor() * depth.lot_size();
            q_qty
        } else {
            0.0
        }
    }
}

/// Stores the values needed for queue position estimation and adjustment for [`ProbQueueModel`].
#[derive(Clone)]
pub struct QueuePos {
    front_q_qty: f32,
    cum_trade_qty: f32,
}

impl AnyClone for QueuePos {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Default for QueuePos {
    fn default() -> Self {
        Self {
            front_q_qty: 0.0,
            cum_trade_qty: 0.0,
        }
    }
}

/// Provides the probability of a decrease behind the order's queue position.
pub trait Probability {
    /// Returns the probability based on the quantity ahead and behind the order.
    fn prob(&self, front: f32, back: f32) -> f32;
}

/// Provides a probability-based queue position model as described in
/// * https://quant.stackexchange.com/questions/3782/how-do-we-estimate-position-of-our-order-in-order-book
/// * https://rigtorp.se/2013/06/08/estimating-order-queue-position.html
///
/// Your order's queue position advances when a trade occurs at the same price level or the quantity
/// at the level decreases. The advancement in queue position depends on the probability based on
/// the relative queue position. To avoid double counting the quantity decrease caused by trades,
/// all trade quantities occurring at the level before the book quantity changes will be subtracted
/// from the book quantity changes.
pub struct ProbQueueModel<P, MD>
where
    P: Probability,
{
    prob: P,
    _md_marker: PhantomData<MD>,
}

impl<P, MD> ProbQueueModel<P, MD>
where
    P: Probability,
{
    /// Constructs an instance of `ProbQueueModel` with a [`Probability`] model.
    pub fn new(prob: P) -> Self {
        Self {
            prob,
            _md_marker: Default::default(),
        }
    }
}

impl<P, MD> QueueModel<MD> for ProbQueueModel<P, MD>
where
    P: Probability,
    MD: MarketDepth,
{
    fn new_order(&self, order: &mut Order, depth: &MD) {
        let mut q = QueuePos::default();
        if order.side == Side::Buy {
            q.front_q_qty = depth.bid_qty_at_tick(order.price_tick);
        } else {
            q.front_q_qty = depth.ask_qty_at_tick(order.price_tick);
        }
        order.q = Box::new(q);
    }

    fn trade(&self, order: &mut Order, qty: f32, _depth: &MD) {
        let q = order.q.as_any_mut().downcast_mut::<QueuePos>().unwrap();
        q.front_q_qty -= qty;
        q.cum_trade_qty += qty;
    }

    fn depth(&self, order: &mut Order, prev_qty: f32, new_qty: f32, _depth: &MD) {
        let mut chg = prev_qty - new_qty;
        // In order to avoid duplicate order queue position adjustment, subtract queue position
        // change by trades.
        let q = order.q.as_any_mut().downcast_mut::<QueuePos>().unwrap();
        chg = chg - q.cum_trade_qty;
        // Reset, as quantity change by trade should be already reflected in qty.
        q.cum_trade_qty = 0.0;
        // For an increase of the quantity, front queue doesn't change by the quantity change.
        if chg < 0.0 {
            q.front_q_qty = q.front_q_qty.min(new_qty);
            return;
        }

        let front = q.front_q_qty;
        let back = prev_qty - front;

        let mut prob = self.prob.prob(front, back);
        if prob.is_infinite() {
            prob = 1.0;
        }

        let est_front = front - (1.0 - prob) * chg + (back - prob * chg).min(0.0);
        q.front_q_qty = est_front.min(new_qty);
    }

    fn is_filled(&self, order: &Order, depth: &MD) -> f32 {
        let q = order.q.as_any().downcast_ref::<QueuePos>().unwrap();
        if (q.front_q_qty / depth.lot_size()).round() < 0.0 {
            let q_qty = (-q.front_q_qty / depth.lot_size()).floor() * depth.lot_size();
            q_qty
        } else {
            0.0
        }
    }
}

/// This probability model uses a power function `f(x) = x ** n` to adjust the probability which is
/// calculated as `f(back) / (f(back) + f(front))`.
pub struct PowerProbQueueFunc {
    n: f32,
}

impl PowerProbQueueFunc {
    /// Constructs an instance of `PowerProbQueueFunc`.
    pub fn new(n: f32) -> Self {
        Self { n }
    }

    fn f(&self, x: f32) -> f32 {
        x.powf(self.n)
    }
}

impl Probability for PowerProbQueueFunc {
    fn prob(&self, front: f32, back: f32) -> f32 {
        self.f(back) / (self.f(back) + self.f(front))
    }
}

/// This probability model uses a logarithmic function `f(x) = log(1 + x)` to adjust the
/// probability which is calculated as `f(back) / (f(back) + f(front))`.
pub struct LogProbQueueFunc(());

impl LogProbQueueFunc {
    /// Constructs an instance of `LogProbQueueFunc`.
    pub fn new() -> Self {
        Self(())
    }

    fn f(&self, x: f32) -> f32 {
        (1.0 + x).ln()
    }
}

impl Probability for LogProbQueueFunc {
    fn prob(&self, front: f32, back: f32) -> f32 {
        self.f(back) / (self.f(back) + self.f(front))
    }
}

/// This probability model uses a logarithmic function `f(x) = log(1 + x)` to adjust the
/// probability which is calculated as `f(back) / f(back + front)`.
pub struct LogProbQueueFunc2(());

impl LogProbQueueFunc2 {
    /// Constructs an instance of `LogProbQueueFunc2`.
    pub fn new() -> Self {
        Self(())
    }

    fn f(&self, x: f32) -> f32 {
        (1.0 + x).ln()
    }
}

impl Probability for LogProbQueueFunc2 {
    fn prob(&self, front: f32, back: f32) -> f32 {
        self.f(back) / self.f(back + front)
    }
}

/// This probability model uses a power function `f(x) = x ** n` to adjust the probability which is
/// calculated as `f(back) / f(back + front)`.
pub struct PowerProbQueueFunc2 {
    n: f32,
}

impl PowerProbQueueFunc2 {
    /// Constructs an instance of `PowerProbQueueFunc2`.
    pub fn new(n: f32) -> Self {
        Self { n }
    }

    fn f(&self, x: f32) -> f32 {
        x.powf(self.n)
    }
}

impl Probability for PowerProbQueueFunc2 {
    fn prob(&self, front: f32, back: f32) -> f32 {
        self.f(back) / self.f(back + front)
    }
}

/// This probability model uses a power function `f(x) = x ** n` to adjust the probability which is
/// calculated as `1 - f(front / (front + back))`.
pub struct PowerProbQueueFunc3 {
    n: f32,
}

impl PowerProbQueueFunc3 {
    /// Constructs an instance of `PowerProbQueueFunc3`.
    pub fn new(n: f32) -> Self {
        Self { n }
    }

    fn f(&self, x: f32) -> f32 {
        x.powf(self.n)
    }
}

impl Probability for PowerProbQueueFunc3 {
    fn prob(&self, front: f32, back: f32) -> f32 {
        1.0 - self.f(front / (front + back))
    }
}

#[cfg(feature = "unstable_l3")]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum L3OrderSource {
    Market,
    Backtesting,
}

#[cfg(feature = "unstable_l3")]
impl AnyClone for L3OrderSource {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(feature = "unstable_l3")]
#[derive(Hash, Eq, PartialEq)]
pub enum L3OrderId {
    Market(i64),
    Backtesting(i64),
}

#[cfg(feature = "unstable_l3")]
impl L3OrderId {
    pub fn is(&self, order: &Order) -> bool {
        let order_source = order.q.as_any().downcast_ref::<L3OrderSource>().unwrap();
        match self {
            L3OrderId::Market(order_id) => {
                order.order_id == *order_id && *order_source == L3OrderSource::Market
            }
            L3OrderId::Backtesting(order_id) => {
                order.order_id == *order_id && *order_source == L3OrderSource::Backtesting
            }
        }
    }
}

#[cfg(feature = "unstable_l3")]
pub trait L3QueueModel {
    type Error;

    fn add_order(&mut self, order_id: L3OrderId, order: Order) -> Result<(), Self::Error>;

    fn cancel_order(&mut self, order_id: L3OrderId) -> Result<Order, Self::Error>;

    fn modify_order(&mut self, order_id: L3OrderId, order: Order) -> Result<(), Self::Error>;

    fn fill(&mut self, order_id: L3OrderId, delete: bool) -> Result<Vec<Order>, Self::Error>;
}

#[cfg(feature = "unstable_l3")]
pub struct L3PriceTimePriorityQueueModel {
    pub orders: HashMap<L3OrderId, (Side, i32)>,
    // Since LinkedList's cursor is still unstable, there is no efficient way to delete an item in a
    // linked list, so it is better to use a vector.
    pub bid_priority: HashMap<i32, Vec<Order>>,
    pub ask_priority: HashMap<i32, Vec<Order>>,
}

#[cfg(feature = "unstable_l3")]
impl L3PriceTimePriorityQueueModel {
    pub fn new() -> Self {
        Self {
            orders: Default::default(),
            bid_priority: Default::default(),
            ask_priority: Default::default(),
        }
    }
}

#[cfg(feature = "unstable_l3")]
impl L3QueueModel for L3PriceTimePriorityQueueModel {
    type Error = BacktestError;

    fn add_order(&mut self, order_id: L3OrderId, order: Order) -> Result<(), Self::Error> {
        let order_price_tick = order.price_tick;
        let side = order.side;

        let priority = match side {
            Side::Buy => self
                .bid_priority
                .entry(order_price_tick)
                .or_insert(Vec::new()),
            Side::Sell => self
                .ask_priority
                .entry(order_price_tick)
                .or_insert(Vec::new()),
            Side::Unsupported => unreachable!(),
        };

        priority.push(order);
        match self.orders.entry(order_id) {
            Entry::Occupied(_) => Err(BacktestError::OrderIdExist),
            Entry::Vacant(entry) => {
                entry.insert((side, order_price_tick));
                Ok(())
            }
        }
    }

    fn cancel_order(&mut self, order_id: L3OrderId) -> Result<Order, Self::Error> {
        let (side, order_price_tick) = self
            .orders
            .remove(&order_id)
            .ok_or(BacktestError::OrderNotFound)?;

        match side {
            Side::Buy => {
                let priority = self.bid_priority.get_mut(&order_price_tick).unwrap();
                let mut pos = None;
                for (i, order_in_q) in priority.iter().enumerate() {
                    if order_id.is(order_in_q) {
                        pos = Some(i);
                        break;
                    }
                }

                let order = priority.remove(pos.ok_or(BacktestError::OrderNotFound)?);
                // if priority.len() == 0 {
                //     self.bid_priority.remove(&order_price_tick);
                // }
                Ok(order)
            }
            Side::Sell => {
                let priority = self.ask_priority.get_mut(&order_price_tick).unwrap();
                let mut pos = None;
                for (i, order_in_q) in priority.iter().enumerate() {
                    if order_id.is(order_in_q) {
                        pos = Some(i);
                        break;
                    }
                }

                let order = priority.remove(pos.ok_or(BacktestError::OrderNotFound)?);
                // if priority.len() == 0 {
                //     self.ask_priority.remove(&order_price_tick);
                // }
                Ok(order)
            }
            Side::Unsupported => unreachable!(),
        }
    }

    fn modify_order(&mut self, order_id: L3OrderId, order: Order) -> Result<(), Self::Error> {
        let (side, order_price_tick) = self
            .orders
            .get(&order_id)
            .ok_or(BacktestError::OrderNotFound)?;

        match side {
            Side::Buy => {
                let priority = self.bid_priority.get_mut(&order_price_tick).unwrap();
                let mut processed = false;
                let mut pos = None;
                for (i, order_in_q) in priority.iter_mut().enumerate() {
                    if order_id.is(order_in_q) {
                        if (order_in_q.price_tick != order.price_tick)
                            || (order_in_q.leaves_qty < order.leaves_qty)
                        {
                            pos = Some(i);
                        } else {
                            order_in_q.leaves_qty = order.leaves_qty;
                            order_in_q.qty = order.qty;
                        }
                        processed = true;
                        break;
                    }
                }

                if !processed {
                    return Err(BacktestError::OrderNotFound);
                }

                if let Some(pos) = pos {
                    let prev_order = priority.remove(pos);
                    // if priority.len() == 0 {
                    //     self.bid_priority.remove(&order_price_tick);
                    // }
                    if prev_order.price_tick != order.price_tick {
                        let priority = self.bid_priority.get_mut(&order.price_tick).unwrap();
                        priority.push(order);
                    } else {
                        priority.push(order);
                    }
                }
            }
            Side::Sell => {
                let priority = self.ask_priority.get_mut(&order_price_tick).unwrap();
                let mut processed = false;
                let mut pos = None;
                for (i, order_in_q) in priority.iter_mut().enumerate() {
                    if order_id.is(order_in_q) {
                        if (order_in_q.price_tick != order.price_tick)
                            || (order_in_q.leaves_qty < order.leaves_qty)
                        {
                            pos = Some(i);
                        } else {
                            order_in_q.leaves_qty = order.leaves_qty;
                            order_in_q.qty = order.qty;
                        }
                        processed = true;
                        break;
                    }
                }

                if !processed {
                    return Err(BacktestError::OrderNotFound);
                }

                if let Some(pos) = pos {
                    let prev_order = priority.remove(pos);
                    // if priority.len() == 0 {
                    //     self.bid_priority.remove(&order_price_tick);
                    // }
                    if prev_order.price_tick != order.price_tick {
                        let priority = self.ask_priority.get_mut(&order.price_tick).unwrap();
                        priority.push(order);
                    } else {
                        priority.push(order);
                    }
                }
            }
            Side::Unsupported => unreachable!(),
        }

        Ok(())
    }

    fn fill(&mut self, order_id: L3OrderId, delete: bool) -> Result<Vec<Order>, Self::Error> {
        let (side, order_price_tick) = self
            .orders
            .remove(&order_id)
            .ok_or(BacktestError::OrderNotFound)?;

        match side {
            Side::Buy => {
                let priority = self.bid_priority.get_mut(&order_price_tick).unwrap();
                let mut filled = Vec::new();

                let mut pos = None;
                let mut i = 0;
                while i < priority.len() {
                    let order_in_q = priority.get(i).unwrap();
                    let order_source = order_in_q
                        .q
                        .as_any()
                        .downcast_ref::<L3OrderSource>()
                        .unwrap();
                    if order_id.is(order_in_q) {
                        pos = Some(i);
                        break;
                    } else if *order_source == L3OrderSource::Backtesting {
                        let order = priority.remove(i);
                        filled.push(order);
                    } else {
                        i += 1;
                    }
                }
                let pos = pos.ok_or(BacktestError::OrderNotFound)?;
                if delete {
                    priority.remove(pos);
                    // if priority.len() == 0 {
                    //     self.ask_priority.remove(&order_price_tick);
                    // }
                }
                Ok(filled)
            }
            Side::Sell => {
                let priority = self.ask_priority.get_mut(&order_price_tick).unwrap();
                let mut filled = Vec::new();

                let mut pos = None;
                let mut i = 0;
                while i < priority.len() {
                    let order_in_q = priority.get(i).unwrap();
                    let order_source = order_in_q
                        .q
                        .as_any()
                        .downcast_ref::<L3OrderSource>()
                        .unwrap();
                    if order_id.is(order_in_q) {
                        pos = Some(i);
                        break;
                    } else if *order_source == L3OrderSource::Backtesting {
                        let order = priority.remove(i);
                        filled.push(order);
                    } else {
                        i += 1;
                    }
                }
                let pos = pos.ok_or(BacktestError::OrderNotFound)?;
                if delete {
                    priority.remove(pos);
                    // if priority.len() == 0 {
                    //     self.ask_priority.remove(&order_price_tick);
                    // }
                }
                Ok(filled)
            }
            Side::Unsupported => unreachable!(),
        }
    }
}
