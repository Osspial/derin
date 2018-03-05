use std::time::{Duration, Instant};
use tree::WidgetID;
use std::cmp;
use std::ops::Range;

pub(crate) struct TimerList {
    last_trigger: Instant,
    timers_by_dist: Vec<Timer>,
    pub rate_limiter: Option<Duration>
}

pub(crate) struct TriggeredTimers<'a> {
    trigger_time: Instant,
    triggered_range: Range<usize>,
    timers_by_dist: &'a mut Vec<Timer>
}

pub struct TimerRegister<'a> {
    widget_id: WidgetID,
    new_timers: Vec<TimerProto>,
    timer_list: &'a mut TimerList
}

struct TimerProto {
    name: &'static str,
    frequency: Duration,
    reset_timer: bool,
    extra_proto: Option<ExtraProto>
}

#[derive(Clone, Copy)]
struct ExtraProto {
    start_time: Instant,
    last_trigger: Instant,
    times_triggered: u64
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Timer {
    pub name: &'static str,
    pub widget_id: WidgetID,
    pub start_time: Instant,
    pub last_trigger: Instant,
    pub frequency: Duration,
    pub times_triggered: u64
}

impl TimerList {
    pub fn new(rate_limiter: Option<Duration>) -> TimerList {
        TimerList {
            last_trigger: Instant::now(),
            timers_by_dist: Vec::new(),
            rate_limiter
        }
    }

    pub fn new_timer_register(&mut self, widget_id: WidgetID) -> TimerRegister {
        TimerRegister {
            widget_id,
            new_timers: Vec::new(),
            timer_list: self
        }
    }

    pub(crate) fn trigger_timers(&mut self) -> TriggeredTimers {
        let trigger_time = Instant::now();
        if (trigger_time - self.last_trigger) < self.rate_limiter.unwrap_or(Duration::new(0, 0)) {
            return TriggeredTimers {
                triggered_range: 0..0,
                trigger_time,
                timers_by_dist: &mut self.timers_by_dist
            };
        }
        let triggered_index = self.timers_by_dist.iter_mut()
            .take_while(|timer| timer.time_until_trigger(trigger_time) == Duration::new(0, 0))
            .enumerate().map(|(i, timer)| {timer.trigger(trigger_time); i + 1})
            .last().unwrap_or(0);

        self.last_trigger = trigger_time;
        TriggeredTimers {
            triggered_range: 0..triggered_index,
            trigger_time,
            timers_by_dist: &mut self.timers_by_dist
        }
    }

    pub fn time_until_trigger(&self) -> Option<Duration> {
        let now = Instant::now();
        self.timers_by_dist.get(0).map(|t|
            cmp::max(
                t.time_until_trigger(now),
                self.rate_limiter.map(|limit| limit - (now - self.last_trigger))
                    .unwrap_or(Duration::new(0, 0))
            )
        )
    }
}

impl<'a> TimerRegister<'a> {
    pub fn add_timer(&mut self, name: &'static str, frequency: Duration, reset_timer: bool) {
        let insert_index = match self.new_timers.binary_search_by_key(&frequency, |t| t.frequency) {
            Ok(i) | Err(i) => i
        };
        self.new_timers.insert(insert_index, TimerProto{ name, frequency, reset_timer, extra_proto: None });
    }
}

impl<'a> Drop for TimerRegister<'a> {
    fn drop(&mut self) {
        let TimerRegister {
            ref mut new_timers,
            widget_id,
            ref mut timer_list
        } = *self;

        // Update any timers that are already in the register.
        timer_list.timers_by_dist.retain(|timer| {
            if timer.widget_id != widget_id {
                return true;
            }

            for new_timer in new_timers.iter_mut() {
                if new_timer.name == timer.name {
                    if !new_timer.reset_timer {
                        new_timer.extra_proto = Some(ExtraProto {
                            start_time: timer.start_time,
                            last_trigger: timer.last_trigger,
                            times_triggered: timer.times_triggered
                        });
                    }
                    break;
                }
            }

            false
        });

        // Add the previously-unregistered timers to the timer register
        let cur_time = Instant::now();
        let mut new_timers = new_timers.drain(..).map(|p| Timer {
            name: p.name,
            widget_id: widget_id,
            start_time: p.extra_proto.map(|p| p.start_time).unwrap_or(cur_time),
            last_trigger: p.extra_proto.map(|p| p.last_trigger).unwrap_or(cur_time - p.frequency),
            frequency: p.frequency,
            times_triggered: p.extra_proto.map(|p| p.times_triggered).unwrap_or(!0) // We default to max value so the addition wraps around to zero
        });
        let mut next_timer: Option<Timer> = new_timers.next();

        let mut index_iter = 0..;
        loop {
            let i = index_iter.next().unwrap();

            let next_timer_ref = match next_timer.as_ref() {
                Some(t) => t,
                None => break
            };

            let timer = match timer_list.timers_by_dist.get(i) {
                Some(timer) => *timer,
                None => Timer::max_frequency(cur_time)
            };

            if next_timer_ref.time_until_trigger(cur_time) < timer.time_until_trigger(cur_time) {
                timer_list.timers_by_dist.insert(i, next_timer.take().unwrap());
                next_timer = new_timers.next();
                index_iter.start += 1;
            }
        }

        debug_assert_eq!(
            timer_list.timers_by_dist,
            {
                let mut sorted = timer_list.timers_by_dist.clone();
                sorted.sort_unstable_by_key(|t| t.time_until_trigger(cur_time));
                sorted
            }
        );
    }
}

impl<'a> TriggeredTimers<'a> {
    pub fn triggered_timers(&self) -> &[Timer] {
        &self.timers_by_dist[self.triggered_range.clone()]
    }
}

impl<'a> Drop for TriggeredTimers<'a> {
    fn drop(&mut self) {
        if self.triggered_range.len() > 0 {
            let trigger_time = self.trigger_time;
            self.timers_by_dist.sort_unstable_by_key(|t| t.time_until_trigger(trigger_time));
        }
    }
}

impl Timer {
    fn max_frequency(cur_time: Instant) -> Timer {
        Timer {
            name: "",
            widget_id: WidgetID::dummy(),
            start_time: cur_time,
            last_trigger: cur_time,
            frequency: Duration::new(!0, 0),
            times_triggered: !0
        }
    }

    fn time_until_trigger(&self, cur_time: Instant) -> Duration {
        self.frequency.checked_sub(cur_time - self.last_trigger).unwrap_or(Duration::new(0, 0))
    }

    fn trigger(&mut self, trigger_time: Instant) {
        self.times_triggered = self.times_triggered.wrapping_add(1);
        self.last_trigger = trigger_time;
    }
}
