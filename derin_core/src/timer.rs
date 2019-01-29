// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::widget::WidgetID;
use std::{
    cell::Cell,
    time::{Instant, Duration},
};

id!(pub TimerID);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timer {
    pub frequency: Duration,
    start_time: Instant,
    pub(crate) last_triggered: Cell<Option<Instant>>,
    pub(crate) times_triggered: Cell<u32>
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct TimerTrigger {
    pub instant: Instant,
    pub timer_id: TimerID,
    pub widget_id: WidgetID,
}

pub(crate) struct TimerTriggerTracker {
    timers_by_next_trigger: Vec<TimerTrigger>,
}

impl Timer {
    pub fn new(frequency: Duration) -> Timer {
        Timer {
            frequency,
            start_time: Instant::now(),
            last_triggered: Cell::new(None),
            times_triggered: Cell::new(0),
        }
    }

    pub fn new_delayed(frequency: Duration, start_time: Instant) -> Timer {
        Timer {
            frequency, start_time,
            last_triggered: Cell::new(None),
            times_triggered: Cell::new(0),
        }
    }

    #[inline(always)]
    pub fn start_time(&self) -> Instant {
        self.start_time
    }
    #[inline(always)]
    pub fn last_triggered(&self) -> Option<Instant> {
        self.last_triggered.get()
    }
    #[inline(always)]
    pub fn times_triggered(&self) -> u32 {
        self.times_triggered.get()
    }

    pub fn next_trigger(&self) -> Instant {
        self.start_time + self.frequency * self.times_triggered()
    }
}

impl TimerTrigger {
    pub fn new(instant: Instant, timer_id: TimerID, widget_id: WidgetID) -> TimerTrigger {
        TimerTrigger{ instant, timer_id, widget_id }
    }
}

impl TimerTriggerTracker {
    pub fn new() -> TimerTriggerTracker {
        TimerTriggerTracker {
            timers_by_next_trigger: Vec::new(),
        }
    }

    pub fn next_trigger(&self) -> Option<Instant> {
        self.timers_by_next_trigger.get(0).map(|t| t.instant)
    }

    pub fn timers_triggered(&mut self) -> impl '_ + Iterator<Item=TimerTrigger> {
        let now = Instant::now();
        let split_location_result = self.timers_by_next_trigger.binary_search_by_key(&now, |t| t.instant);
        let split_location = match split_location_result {
            Ok(i) => {
                // If there are multiple timers triggered at now, find the last timer in that set.
                i + self.timers_by_next_trigger[i..].iter().take_while(|t| t.instant == now).count()
            }
            Err(i) => i
        };

        self.timers_by_next_trigger[..split_location].sort_unstable_by_key(|t| t.widget_id);
        self.timers_by_next_trigger.drain(..split_location)
    }

    pub fn queue_trigger(&mut self, timer_trigger: TimerTrigger) {
        let insert_location_result = self.timers_by_next_trigger.binary_search(&timer_trigger);
        let insert_location = match insert_location_result {
            Ok(_) => return,
            Err(i) => i
        };

        self.timers_by_next_trigger.insert(insert_location, timer_trigger);
    }
}
