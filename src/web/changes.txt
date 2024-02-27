use differential_dataflow::trace::TraceReader;
use differential_dataflow::trace::BatchReader;
use differential_dataflow::trace::Cursor;

use std::fmt::Debug;

use crate::web::messaging::ChangeMessage;
use crate::web::types::Trace;

pub fn collect_diffs<K, V, F>(
    trace: Trace<K, V>,
    time_of_interest: usize,
    logic: F,
) -> Vec<ChangeMessage>
    where V: Clone + Ord + Debug,
          K: Clone + Ord + Debug,
          F: Fn(&K, &V, usize, isize) -> ChangeMessage + 'static
{
    let mut messages = Vec::new();

    trace.map_batches(|batch| {
        //if batch.lower().iter().find(|t| *(*t) == time_of_interest) != None {

        let mut cursor = batch.cursor();

        while cursor.key_valid(&batch) {
            while cursor.val_valid(&batch) {

                let key = cursor.key(&batch);
                let value = cursor.val(&batch);

                cursor.map_times(&batch, |time, diff| {
                    if *time == time_of_interest {
                        messages.push(logic(&key, &value, *time, *diff));
                    }
                });

                cursor.step_val(&batch);
            }
            cursor.step_key(&batch);
        }
        //}
    });

    //trace.distinguish_since(&[]);
    //trace.advance_by(&[time_of_interest]);

    messages
}