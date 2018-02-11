use std::time::{Duration, Instant};
use tree::NodeID;

pub(crate) struct TimerList {
    start_time: Instant,
    last_trigger: Instant,
    timers_by_dist: Vec<Timer>,
    pub rate_limiter: Option<Duration>
}

pub(crate) struct TimerIter {

}

pub struct TimerRegister<'a> {
    node_id: NodeID,
    new_timers: Vec<TimerProto>,
    timer_list: &'a mut TimerList
}

struct TimerProto {
    name: &'static str,
    frequency: Duration,
}

#[derive(Debug, Clone)]
struct Timer {
    name: &'static str,
    node_id: NodeID,
    start_time: Instant,
    last_trigger: Instant,
    frequency: Duration,
    times_triggered: u64
}

impl TimerList {
    pub fn new(rate_limiter: Option<Duration>) -> TimerList {
        let cur_time = Instant::now();
        TimerList {
            start_time: cur_time,
            last_trigger: cur_time,
            timers_by_dist: Vec::new(),
            rate_limiter
        }
    }

    pub fn new_timer_register(&mut self, node_id: NodeID) -> TimerRegister {
        TimerRegister {
            node_id,
            new_timers: Vec::new(),
            timer_list: self
        }
    }
}

impl<'a> TimerRegister<'a> {
    pub fn add_timer(&mut self, name: &'static str, frequency: Duration) {
        let insert_index = match self.timer_list.timers_by_dist.binary_search_by_key(&frequency, |t| t.frequency) {
            Ok(i) | Err(i) => i
        };
        self.new_timers.insert(insert_index, TimerProto{ name, frequency });
    }
}

impl<'a> Drop for TimerRegister<'a> {
    fn drop(&mut self) {
        let TimerRegister {
            ref mut new_timers,
            node_id,
            ref mut timer_list
        } = *self;

        let cur_time = Instant::now();
        let mut new_timers = new_timers.drain(..).map(|p| Timer {
            ..unimplemented!()
        //     name: p.name,
        //     node_id: node_id,
        //     start_time: cur_time,
        //     last_trigger: cur_time,
        //     frequency: p.frequency,
        //     times_triggered: 0
        }).peekable();

        // for i in 0.. {
        //     let timer = match timer_list.timers_by_dist.get(i) {
        //         Some(timer) => timer,
        //         None => break
        //     };

        //     let next_timer = match new_timers.peek() {
        //         Some(next_timer) => next_timer,
        //         None => break
        //     };

        //     let dur_zero = Duration::new(0, 0);
        //     if next_timer.time_until_trigger(cur_time).unwrap_or(dur_zero) < timer.time_until_trigger(cur_time).unwrap_or(dur_zero) {
        //         timer_list.timers_by_dist.push(new_timers.next().unwrap());
        //     }
        // }
    }
}

impl Timer {
    fn time_until_trigger(&self, cur_time: Instant) -> Option<Duration> {
        self.frequency.checked_sub(cur_time - self.last_trigger)
    }
}
