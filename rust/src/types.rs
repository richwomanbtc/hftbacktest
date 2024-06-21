use std::{
    any::Any,
    collections::HashMap,
    fmt::{Debug, Formatter},
    sync::Arc,
};

use dyn_clone::DynClone;
use thiserror::Error;

use crate::depth::MarketDepth;

/// Error type assigned to [`Error`].
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[repr(i64)]
pub enum ErrorKind {
    ConnectionInterrupted = 0,
    CriticalConnectionError = 1,
    OrderError = 2,
    Custom(i64),
}

/// Error conveyed through [`LiveEvent`].
#[derive(Clone, Debug)]
pub struct Error {
    pub kind: ErrorKind,
    pub value: Option<Arc<Box<dyn Any + Send + Sync>>>,
}

impl Error {
    /// Constructs an instance of `Error`.
    pub fn new(kind: ErrorKind) -> Error {
        Self { kind, value: None }
    }

    /// Constructs an instance of `Error` with a value that is either the original error or contains
    /// detailed error information.
    pub fn with<T>(kind: ErrorKind, value: T) -> Error
    where
        T: Send + Sync + 'static,
    {
        Self {
            kind,
            value: Some(Arc::new(Box::new(value))),
        }
    }

    /// Returns some reference to the value if it exists and is of type `T`, or `None` if it isn’t.
    pub fn value_downcast_ref<T>(&self) -> Option<&T>
    where
        T: 'static,
    {
        self.value
            .as_ref()
            .map(|value| value.downcast_ref())
            .flatten()
    }
}

/// Events occurring in a live bot sent by a [`Connector`](`crate::connector::Connector`).
#[derive(Clone, Debug)]
pub enum LiveEvent {
    L2Feed { asset_no: usize, events: Vec<Event> },
    L3Feed { asset_no: usize, event: L3Event },
    Order { asset_no: usize, order: Order },
    Position { asset_no: usize, qty: f64 },
    Error(Error),
}

/// Indicates a buy, with specific meaning that can vary depending on the situation. For example,
/// when combined with a depth event, it means a bid-side event, while when combined with a trade
/// event, it means that the trade initiator is a buyer.
pub const BUY: i64 = 1 << 29;

/// Indicates a sell, with specific meaning that can vary depending on the situation. For example,
/// when combined with a depth event, it means an ask-side event, while when combined with a trade
/// event, it means that the trade initiator is a seller.
pub const SELL: i64 = 1 << 28;

pub const SIDE: i64 = BUY | SELL;

/// Indicates that the market depth is changed.
pub const DEPTH_EVENT: i64 = 1;

/// Indicates that a trade occurs in the market.
pub const TRADE_EVENT: i64 = 2;

/// Indicates that the market depth is cleared.
pub const DEPTH_CLEAR_EVENT: i64 = 3;

/// Indicates that the market depth snapshot is received.
pub const DEPTH_SNAPSHOT_EVENT: i64 = 4;

#[cfg(feature = "unstable_l3")]
pub const ADD_ORDER_EVENT: i64 = 10;

#[cfg(feature = "unstable_l3")]
pub const CANCEL_ORDER_EVENT: i64 = 11;

#[cfg(feature = "unstable_l3")]
pub const MODIFY_ORDER_EVENT: i64 = 12;

#[cfg(feature = "unstable_l3")]
pub const FILL_EVENT: i64 = 13;

/// Indicates that it is a valid event to be handled by the exchange processor at the exchange
/// timestamp.
pub const EXCH_EVENT: i64 = 1 << 31;

/// Indicates that it is a valid event to be handled by the local processor at the local timestamp.
pub const LOCAL_EVENT: i64 = 1 << 30;

/// Represents a combination of a [`DEPTH_EVENT`], [`BUY`], and `LOCAL_EVENT`.
pub const LOCAL_BID_DEPTH_EVENT: i64 = DEPTH_EVENT | BUY | LOCAL_EVENT;

/// Represents a combination of [`DEPTH_EVENT`], [`SELL`], and `LOCAL_EVENT`.
pub const LOCAL_ASK_DEPTH_EVENT: i64 = DEPTH_EVENT | SELL | LOCAL_EVENT;

/// Represents a combination of [`DEPTH_CLEAR_EVENT`], [`BUY`], and `LOCAL_EVENT`.
pub const LOCAL_BID_DEPTH_CLEAR_EVENT: i64 = DEPTH_CLEAR_EVENT | BUY | LOCAL_EVENT;

/// Represents a combination of [`DEPTH_CLEAR_EVENT`], [`SELL`], and `LOCAL_EVENT`.
pub const LOCAL_ASK_DEPTH_CLEAR_EVENT: i64 = DEPTH_CLEAR_EVENT | SELL | LOCAL_EVENT;

/// Represents a combination of [`DEPTH_SNAPSHOT_EVENT`], [`BUY`], and `LOCAL_EVENT`.
pub const LOCAL_BID_DEPTH_SNAPSHOT_EVENT: i64 = DEPTH_SNAPSHOT_EVENT | BUY | LOCAL_EVENT;

/// Represents a combination of [`DEPTH_SNAPSHOT_EVENT`], [`SELL`], and `LOCAL_EVENT`.
pub const LOCAL_ASK_DEPTH_SNAPSHOT_EVENT: i64 = DEPTH_SNAPSHOT_EVENT | SELL | LOCAL_EVENT;

/// Represents a combination of [`TRADE_EVENT`], and `LOCAL_EVENT`.
pub const LOCAL_TRADE_EVENT: i64 = TRADE_EVENT | LOCAL_EVENT;

/// Represents a combination of [`TRADE_EVENT`], [`SELL`], and `LOCAL_EVENT`.
pub const LOCAL_BUY_TRADE_EVENT: i64 = LOCAL_TRADE_EVENT | BUY;

/// Represents a combination of [`TRADE_EVENT`], [`BUY`], and `LOCAL_EVENT`.
pub const LOCAL_SELL_TRADE_EVENT: i64 = LOCAL_TRADE_EVENT | SELL;

/// Represents a combination of [`DEPTH_EVENT`], [`BUY`], and `EXCH_EVENT`.
pub const EXCH_BID_DEPTH_EVENT: i64 = DEPTH_EVENT | BUY | EXCH_EVENT;

/// Represents a combination of [`DEPTH_EVENT`], [`SELL`], and `EXCH_EVENT`.
pub const EXCH_ASK_DEPTH_EVENT: i64 = DEPTH_EVENT | SELL | EXCH_EVENT;

/// Represents a combination of [`DEPTH_CLEAR_EVENT`], [`BUY`], and `EXCH_EVENT`.
pub const EXCH_BID_DEPTH_CLEAR_EVENT: i64 = DEPTH_CLEAR_EVENT | BUY | EXCH_EVENT;

/// Represents a combination of [`DEPTH_CLEAR_EVENT`], [`SELL`], and `EXCH_EVENT`.
pub const EXCH_ASK_DEPTH_CLEAR_EVENT: i64 = DEPTH_CLEAR_EVENT | SELL | EXCH_EVENT;

/// Represents a combination of [`DEPTH_SNAPSHOT_EVENT`], [`BUY`], and `EXCH_EVENT`.
pub const EXCH_BID_DEPTH_SNAPSHOT_EVENT: i64 = DEPTH_SNAPSHOT_EVENT | BUY | EXCH_EVENT;

/// Represents a combination of [`DEPTH_SNAPSHOT_EVENT`], [`SELL`], and `EXCH_EVENT`.
pub const EXCH_ASK_DEPTH_SNAPSHOT_EVENT: i64 = DEPTH_SNAPSHOT_EVENT | SELL | EXCH_EVENT;

/// Represents a combination of [`TRADE_EVENT`], and `EXCH_EVENT`.
pub const EXCH_TRADE_EVENT: i64 = TRADE_EVENT | EXCH_EVENT;

/// Represents a combination of [`TRADE_EVENT`], [`BUY`], and `EXCH_EVENT`.
pub const EXCH_BUY_TRADE_EVENT: i64 = EXCH_TRADE_EVENT | BUY;

/// Represents a combination of [`TRADE_EVENT`], [`SELL`], and `EXCH_EVENT`.
pub const EXCH_SELL_TRADE_EVENT: i64 = EXCH_TRADE_EVENT | SELL;

#[cfg(feature = "unstable_l3")]
pub const LOCAL_ADD_ORDER_EVENT: i64 = LOCAL_EVENT | ADD_ORDER_EVENT;

#[cfg(feature = "unstable_l3")]
pub const LOCAL_BID_ADD_ORDER_EVENT: i64 = BUY | LOCAL_ADD_ORDER_EVENT;

#[cfg(feature = "unstable_l3")]
pub const LOCAL_ASK_ADD_ORDER_EVENT: i64 = SELL | LOCAL_ADD_ORDER_EVENT;

#[cfg(feature = "unstable_l3")]
pub const LOCAL_CANCEL_ORDER_EVENT: i64 = LOCAL_EVENT | CANCEL_ORDER_EVENT;

#[cfg(feature = "unstable_l3")]
pub const LOCAL_MODIFY_ORDER_EVENT: i64 = LOCAL_EVENT | MODIFY_ORDER_EVENT;

#[cfg(feature = "unstable_l3")]
pub const LOCAL_FILL_EVENT: i64 = LOCAL_EVENT | FILL_EVENT;

#[cfg(feature = "unstable_l3")]
pub const EXCH_ADD_ORDER_EVENT: i64 = EXCH_EVENT | ADD_ORDER_EVENT;

#[cfg(feature = "unstable_l3")]
pub const EXCH_BID_ADD_ORDER_EVENT: i64 = BUY | EXCH_ADD_ORDER_EVENT;

#[cfg(feature = "unstable_l3")]
pub const EXCH_ASK_ADD_ORDER_EVENT: i64 = SELL | EXCH_ADD_ORDER_EVENT;

#[cfg(feature = "unstable_l3")]
pub const EXCH_CANCEL_ORDER_EVENT: i64 = EXCH_EVENT | CANCEL_ORDER_EVENT;

#[cfg(feature = "unstable_l3")]
pub const EXCH_MODIFY_ORDER_EVENT: i64 = EXCH_EVENT | MODIFY_ORDER_EVENT;

#[cfg(feature = "unstable_l3")]
pub const EXCH_FILL_EVENT: i64 = EXCH_EVENT | FILL_EVENT;

/// Indicates that one should not wait for an order response.
pub const WAIT_ORDER_RESPONSE_NONE: i64 = -1;

/// Indicates that one should wait for any order response.
pub const WAIT_ORDER_RESPONSE_ANY: i64 = -2;

/// Indicates that one should continue until the end of the data.
pub const UNTIL_END_OF_DATA: i64 = i64::MAX;

/// Exchange event data.
#[derive(Clone, PartialEq, Debug)]
#[repr(C, align(32))]
pub struct Event {
    /// Event flag
    pub ev: i64,
    /// Exchange timestamp, which is the time at which the event occurs on the exchange.
    pub exch_ts: i64,
    /// Exchange timestamp, which is the time at which the event occurs on the local.
    pub local_ts: i64,
    /// Price
    pub px: f32,
    /// Quantity
    pub qty: f32,
}

impl Event {
    /// Checks if this `Event` corresponds to the given event.
    #[inline(always)]
    pub fn is(&self, event: i64) -> bool {
        if (self.ev & event) != event {
            false
        } else {
            let event_kind = event & 0xff;
            if event_kind == 0 {
                true
            } else {
                self.ev & 0xff == event_kind
            }
        }
    }
}

/// Exchange Level3 Market-By-Order event data.
#[derive(Clone, PartialEq, Debug)]
#[repr(C, align(64))]
pub struct L3Event {
    /// Event flag
    pub ev: i64,
    /// Exchange timestamp, which is the time at which the event occurs on the exchange.
    pub exch_ts: i64,
    /// Exchange timestamp, which is the time at which the event occurs on the local.
    pub local_ts: i64,
    /// Order Id
    pub order_id: i64,
    /// Price
    pub px: f32,
    /// Quantity
    pub qty: f32,
    /// Priority, which is required when the order book needs to be recovered from the snapshot.
    pub priority: u64,
    pub _reserved: [i64; 2],
}

impl L3Event {
    /// Checks if this `L3Event` corresponds to the given event.
    #[inline(always)]
    pub fn is(&self, event: i64) -> bool {
        if (self.ev & event) != event {
            false
        } else {
            let event_kind = event & 0xff;
            if event_kind == 0 {
                true
            } else {
                self.ev & 0xff == event_kind
            }
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[repr(i8)]
pub enum Side {
    /// In the market depth event, this indicates the bid side; in the market trade event, it
    /// indicates that the trade initiator is a buyer.
    Buy = 1,
    /// In the market depth event, this indicates the ask side; in the market trade event, it
    /// indicates that the trade initiator is a seller.
    Sell = -1,
    /// This occurs when the [`Connector`](`crate::connector::Connector`) receives a side value that
    /// does not have a corresponding enum value.
    Unsupported = 127,
}

impl Side {
    /// Returns `1` if this is a [`Buy`], `-1` if this is a `Sell`; otherwise, it will panic.
    pub fn as_f64(&self) -> f64 {
        match self {
            Side::Buy => 1f64,
            Side::Sell => -1f64,
            Side::Unsupported => panic!("Side::Unsupported"),
        }
    }

    /// Returns `1` if this is a [`Buy`], `-1` if this is a `Sell`; otherwise, it will panic.
    pub fn as_f32(&self) -> f32 {
        match self {
            Side::Buy => 1f32,
            Side::Sell => -1f32,
            Side::Unsupported => panic!("Side::Unsupported"),
        }
    }
}

/// Side
impl AsRef<str> for Side {
    fn as_ref(&self) -> &'static str {
        match self {
            Side::Buy => "BUY",
            Side::Sell => "SELL",
            Side::Unsupported => panic!("Side::Unsupported"),
        }
    }
}

/// Order status
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[repr(u8)]
pub enum Status {
    None = 0,
    New = 1,
    Expired = 2,
    Filled = 3,
    Canceled = 4,
    PartiallyFilled = 5,
    /// This occurs when the [`Connector`](`crate::connector::Connector`) receives an order status
    /// value that does not have a corresponding enum value.
    Unsupported = 255,
}

/// Time In Force
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[repr(u8)]
pub enum TimeInForce {
    /// Good 'Til Canceled
    GTC = 0,
    /// Post-only
    GTX = 1,
    /// Fill or Kill
    FOK = 2,
    /// Immediate or Cancel
    IOC = 3,
    /// This occurs when the [`Connector`](`crate::connector::Connector`) receives a time-in-force
    /// value that does not have a corresponding enum value.
    Unsupported = 255,
}

impl AsRef<str> for TimeInForce {
    fn as_ref(&self) -> &'static str {
        match self {
            TimeInForce::GTC => "GTC",
            TimeInForce::GTX => "GTX",
            TimeInForce::FOK => "FOK",
            TimeInForce::IOC => "IOC",
            TimeInForce::Unsupported => panic!("TimeInForce::Unsupported"),
        }
    }
}

/// Order type
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
#[repr(u8)]
pub enum OrdType {
    Limit = 0,
    Market = 1,
    Unsupported = 255,
}

impl AsRef<str> for OrdType {
    fn as_ref(&self) -> &'static str {
        match self {
            OrdType::Limit => "LIMIT",
            OrdType::Market => "MARKET",
            OrdType::Unsupported => panic!("OrdType::Unsupported"),
        }
    }
}

/// Provides cloning of `Box<dyn Any>`, which is utilized in [Order] for the additional data used in
/// [`QueueModel`](`crate::backtest::models::QueueModel`).
///
/// **Usage:**
/// ```
/// impl AnyClone for QueuePos {
///     fn as_any(&self) -> &dyn Any {
///         self
///     }
///
///     fn as_any_mut(&mut self) -> &mut dyn Any {
///         self
///     }
/// }
/// ```
pub trait AnyClone: DynClone {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
dyn_clone::clone_trait_object!(AnyClone);

impl AnyClone for () {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Order
#[derive(Clone)]
pub struct Order {
    /// Order quantity
    pub qty: f32,
    /// The quantity of this order that has not yet been executed. It represents the remaining
    /// quantity that is still open or active in the market after any partial fills.
    pub leaves_qty: f32,
    /// Order price in ticks (`price / tick_size`).
    pub price_tick: i32,
    /// The tick size of the asset associated with this order.
    pub tick_size: f32,
    pub side: Side,
    pub time_in_force: TimeInForce,
    /// The time at which the exchange processes this order, ideally when the matching engine
    /// processes the order, will be set if the value is available.
    pub exch_timestamp: i64,
    pub status: Status,
    /// The time at which the local receives this order or sent this order to the exchange.
    pub local_timestamp: i64,
    /// Request status:
    ///   * [`Status::New`]: Request to open a new order.
    ///   * [`Status::Canceled`]: Request to cancel an opened order.
    pub req: Status,
    /// Executed price in ticks (`executed_price / tick_size`), only available when this order is
    /// executed.
    pub exec_price_tick: i32,
    /// Executed quantity, only available when this order is executed.
    pub exec_qty: f32,
    pub order_id: i64,
    /// Additional data used for [`QueueModel`](`crate::backtest::models::QueueModel`).
    /// This is only available in backtesting, and the type `Q` is set to `()` in a live bot.
    pub q: Box<dyn AnyClone + Send>,
    /// Whether the order is executed as a maker, only available when this order is executed.
    pub maker: bool,
    pub order_type: OrdType,
}

impl Order {
    /// Constructs an instance of `Order`.
    pub fn new(
        order_id: i64,
        price_tick: i32,
        tick_size: f32,
        qty: f32,
        side: Side,
        order_type: OrdType,
        time_in_force: TimeInForce,
    ) -> Self {
        Self {
            qty,
            leaves_qty: qty,
            price_tick,
            tick_size,
            side,
            time_in_force,
            exch_timestamp: 0,
            status: Status::None,
            local_timestamp: 0,
            req: Status::None,
            exec_price_tick: 0,
            exec_qty: 0.0,
            order_id,
            q: Box::new(()),
            maker: false,
            order_type,
        }
    }

    /// Returns the order price.
    pub fn price(&self) -> f32 {
        self.price_tick as f32 * self.tick_size
    }

    /// Returns the executed price, only available when this order is executed.
    pub fn exec_price(&self) -> f32 {
        self.exec_price_tick as f32 * self.tick_size
    }

    /// Returns whether this order is cancelable.
    pub fn cancellable(&self) -> bool {
        (self.status == Status::New || self.status == Status::PartiallyFilled)
            && self.req == Status::None
    }

    /// Returns whether this order is active in the market.
    pub fn active(&self) -> bool {
        self.status == Status::New || self.status == Status::PartiallyFilled
    }

    /// Returns whether this order has an ongoing request.
    pub fn pending(&self) -> bool {
        self.req != Status::None
    }

    /// Updates this order with the given order. This is used only by the processor in backtesting
    /// or by a bot in live trading.
    pub fn update(&mut self, order: &Order) {
        self.qty = order.qty;
        self.leaves_qty = order.leaves_qty;
        self.price_tick = order.price_tick;
        self.tick_size = order.tick_size;
        self.side = order.side;
        self.time_in_force = order.time_in_force;

        assert!(order.exch_timestamp >= self.exch_timestamp);
        if order.exch_timestamp > 0 {
            self.exch_timestamp = order.exch_timestamp;
        }
        self.status = order.status;
        if order.local_timestamp > 0 {
            self.local_timestamp = order.local_timestamp;
        }
        self.req = order.req;
        self.exec_price_tick = order.exec_price_tick;
        self.exec_qty = order.exec_qty;
        self.order_id = order.order_id;
        self.q = order.q.clone();
        self.maker = order.maker;
        self.order_type = order.order_type;
    }
}

impl Debug for Order {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Order")
            .field("qty", &self.qty)
            .field("leaves_qty", &self.leaves_qty)
            .field("price_tick", &self.price_tick)
            .field("tick_size", &self.tick_size)
            .field("side", &self.side)
            .field("time_in_force", &self.time_in_force)
            .field("exch_timestamp", &self.exch_timestamp)
            .field("status", &self.status)
            .field("local_timestamp", &self.local_timestamp)
            .field("req", &self.req)
            .field("exec_price_tick", &self.exec_price_tick)
            .field("exec_qty", &self.exec_qty)
            .field("order_id", &self.order_id)
            .field("maker", &self.maker)
            .field("order_type", &self.order_type)
            .finish()
    }
}

/// An asynchronous request to [`Connector`](`crate::connector::Connector`).
#[derive(Clone, Debug)]
pub enum Request {
    /// An order request, a tuple consisting of an asset number and an [`Order`].
    Order { asset_no: usize, order: Order },
}

/// Provides state values.
#[derive(PartialEq, Clone, Debug)]
pub struct StateValues {
    pub position: f64,
    pub balance: f64,
    pub fee: f64,
    pub trade_num: i32,
    pub trade_qty: f64,
    pub trade_amount: f64,
}

/// Provides errors that can occur in builders.
#[derive(Error, Debug)]
pub enum BuildError {
    #[error("`{0}` is required")]
    BuilderIncomplete(&'static str),
    #[error("{0}")]
    InvalidArgument(&'static str),
    #[error("`{0}/{1}` already exists")]
    Duplicate(String, String),
    #[error("`{0}` is not found")]
    ConnectorNotFound(String),
    #[error("{0:?}")]
    Error(#[from] anyhow::Error),
}

pub struct OrderRequest {
    pub order_id: i64,
    pub price: f32,
    pub qty: f32,
    pub side: Side,
    pub time_in_force: TimeInForce,
    pub order_type: OrdType,
}

/// Provides an interface for a backtester or a bot.
pub trait Bot {
    type Error;

    /// In backtesting, this timestamp reflects the time at which the backtesting is conducted
    /// within the provided data. In a live bot, it's literally the current local timestamp.
    fn current_timestamp(&self) -> i64;

    /// Returns the number of assets.
    fn num_assets(&self) -> usize;

    /// Returns the position you currently hold.
    ///
    /// * `asset_no` - Asset number from which the position will be retrieved.
    fn position(&self, asset_no: usize) -> f64;

    /// Returns the state's values such as balance, fee, and so on.
    fn state_values(&self, asset_no: usize) -> StateValues;

    /// Returns the [MarketDepth](crate::depth::MarketDepth).
    ///
    /// * `asset_no` - Asset number from which the market depth will be retrieved.
    fn depth(&self, asset_no: usize) -> &dyn MarketDepth;

    /// Returns the last market trades.
    ///
    /// * `asset_no` - Asset number from which the last market trades will be retrieved.
    fn trade(&self, asset_no: usize) -> Vec<&dyn Any>;

    /// Clears the last market trades from the buffer.
    ///
    /// * `asset_no` - Asset number at which this command will be executed. If `None`, all last
    ///                trades in any assets will be cleared.
    fn clear_last_trades(&mut self, asset_no: Option<usize>);

    /// Returns a hash map of order IDs and their corresponding [`Order`]s.
    ///
    /// * `asset_no` - Asset number from which orders will be retrieved.
    fn orders(&self, asset_no: usize) -> &HashMap<i64, Order>;

    /// Places a buy order.
    ///
    /// * `asset_no` - Asset number at which this command will be executed.
    /// * `order_id` - The unique order ID; there should not be any existing order with the same ID
    ///                on both local and exchange sides.
    /// * `price` - Order price.
    /// * `qty` - Quantity to buy.
    /// * `time_in_force` - Available [`TimeInForce`] options vary depending on the exchange model.
    ///                     See to the exchange model for details.
    ///
    ///  * `order_type` - Available [`OrdType`] options vary depending on the exchange model. See to
    ///                   the exchange model for details.
    ///
    ///  * `wait` - If true, wait until the order placement response is received.
    fn submit_buy_order(
        &mut self,
        asset_no: usize,
        order_id: i64,
        price: f32,
        qty: f32,
        time_in_force: TimeInForce,
        order_type: OrdType,
        wait: bool,
    ) -> Result<bool, Self::Error>;

    /// Places a sell order.
    ///
    /// * `asset_no` - Asset number at which this command will be executed.
    /// * `order_id` - The unique order ID; there should not be any existing order with the same ID
    ///                on both local and exchange sides.
    /// * `price` - Order price.
    /// * `qty` - Quantity to buy.
    /// * `time_in_force` - Available [`TimeInForce`] options vary depending on the exchange model.
    ///                     See to the exchange model for details.
    ///
    ///  * `order_type` - Available [`OrdType`] options vary depending on the exchange model. See to
    ///                   the exchange model for details.
    ///
    ///  * `wait` - If true, wait until the order placement response is received.
    fn submit_sell_order(
        &mut self,
        asset_no: usize,
        order_id: i64,
        price: f32,
        qty: f32,
        time_in_force: TimeInForce,
        order_type: OrdType,
        wait: bool,
    ) -> Result<bool, Self::Error>;

    /// Places an order.
    fn submit_order(
        &mut self,
        asset_no: usize,
        order: OrderRequest,
        wait: bool,
    ) -> Result<bool, Self::Error>;

    /// Cancels the specified order.
    ///
    /// * `asset_no` - Asset number at which this command will be executed.
    /// * `order_id` - Order ID to cancel.
    /// * `wait` - If true, wait until the order placement response is received.
    fn cancel(&mut self, asset_no: usize, order_id: i64, wait: bool) -> Result<bool, Self::Error>;

    /// Clears inactive orders from the local orders whose status is neither [`Status::New`] nor
    /// [`Status::PartiallyFilled`].
    fn clear_inactive_orders(&mut self, asset_no: Option<usize>);

    /// Waits for the response of the order with the given order ID until timeout.
    fn wait_order_response(
        &mut self,
        asset_no: usize,
        order_id: i64,
        timeout: i64,
    ) -> Result<bool, Self::Error>;

    /// Wait until the next feed is received, or until timeout.
    fn wait_next_feed(
        &mut self,
        include_order_resp: bool,
        timeout: i64,
    ) -> Result<bool, Self::Error>;

    /// Elapses the specified duration.
    ///
    /// Args:
    /// * `duration` - Duration to elapse. Nanoseconds is the default unit. However, unit should be
    ///                the same as the data's timestamp unit.
    ///
    /// Returns:
    ///   `Ok(true)` if the method reaches the specified timestamp within the data. If the end of
    ///   the data is reached before the specified timestamp, it returns `Ok(false)`.
    fn elapse(&mut self, duration: i64) -> Result<bool, Self::Error>;

    /// Elapses time only in backtesting. In live mode, it is ignored.
    ///
    /// The [elapse()](Self::elapse()) method exclusively manages time during backtesting, meaning
    /// that factors such as computing time are not properly accounted for. So, this method can be
    /// utilized to simulate such processing times.
    ///
    /// Args:
    /// * `duration` - Duration to elapse. Nanoseconds is the default unit. However, unit should be
    ///                the same as the data's timestamp unit.
    ///
    /// Returns:
    ///   `Ok(true)` if the method reaches the specified timestamp within the data. If the end of
    ///   the data is reached before the specified timestamp, it returns `Ok(false)`.
    fn elapse_bt(&mut self, duration: i64) -> Result<bool, Self::Error>;

    /// Closes this backtester or bot.
    fn close(&mut self) -> Result<(), Self::Error>;

    /// Returns the last feed's exchange timestamp and local receipt timestamp.
    fn feed_latency(&self, asset_no: usize) -> Option<(i64, i64)>;

    /// Returns the last order's request timestamp, exchange timestamp, and response receipt
    /// timestamp.
    fn order_latency(&self, asset_no: usize) -> Option<(i64, i64, i64)>;
}

/// Provides an interface for a backtester or a bot.
pub trait BotTypedDepth<MD> {
    /// Returns the [MarketDepth](crate::depth::MarketDepth).
    ///
    /// * `asset_no` - Asset number from which the market depth will be retrieved.
    fn depth_typed(&self, asset_no: usize) -> &MD;
}

/// Provides an interface for a backtester or a bot.
pub trait BotTypedTrade<Event> {
    /// Returns the last market trades.
    ///
    /// * `asset_no` - Asset number from which the last market trades will be retrieved.
    fn trade_typed(&self, asset_no: usize) -> &Vec<Event>;
}

pub trait Recorder {
    type Error;
    fn record<MD, I>(&mut self, hbt: &mut I) -> Result<(), Self::Error>
    where
        I: Bot + BotTypedDepth<MD>,
        MD: MarketDepth;
}

#[cfg(test)]
mod test {
    use crate::{
        prelude::LOCAL_EVENT,
        types::{
            Event,
            BUY,
            LOCAL_BID_DEPTH_CLEAR_EVENT,
            LOCAL_BID_DEPTH_EVENT,
            LOCAL_BID_DEPTH_SNAPSHOT_EVENT,
            LOCAL_BUY_TRADE_EVENT,
        },
    };

    #[test]
    fn test_event_is() {
        let event = Event {
            ev: LOCAL_BID_DEPTH_CLEAR_EVENT | (1 << 20),
            exch_ts: 0,
            local_ts: 0,
            px: 0.0,
            qty: 0.0,
        };

        assert!(!event.is(LOCAL_BID_DEPTH_EVENT));
        assert!(!event.is(LOCAL_BUY_TRADE_EVENT));
        assert!(event.is(LOCAL_BID_DEPTH_CLEAR_EVENT));

        let event = Event {
            ev: LOCAL_EVENT | BUY | 0xff,
            exch_ts: 0,
            local_ts: 0,
            px: 0.0,
            qty: 0.0,
        };

        assert!(!event.is(LOCAL_BID_DEPTH_EVENT));
        assert!(!event.is(LOCAL_BUY_TRADE_EVENT));
        assert!(!event.is(LOCAL_BID_DEPTH_CLEAR_EVENT));
        assert!(!event.is(LOCAL_BID_DEPTH_SNAPSHOT_EVENT));
        assert!(event.is(LOCAL_EVENT));
        assert!(event.is(BUY));
    }
}
