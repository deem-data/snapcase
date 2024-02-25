use differential_dataflow::input::InputSession;
use differential_dataflow::operators::arrange::ArrangeByKey;

use timely::dataflow::operators::Probe;
use timely::dataflow::operators::probe::Handle;
use timely::dataflow::ProbeHandle;
use timely::communication::allocator::thread::Thread;
use timely::worker::Worker;

use crate::web::types::Trace;

pub fn dataflow(
    mut worker: Worker<Thread>
) -> (InputSession<usize, (usize, usize), isize>,
      ProbeHandle<usize>,
      Trace<usize, usize>,
) {

    let mut input = InputSession::new();
    let mut probe = Handle::new();

    let trace = worker.dataflow(|scope| {

        let things = input.to_collection(scope);
        let arranged_things = things
            .map(|(k, v)| (k, v * 2_usize))
            .arrange_by_key();

        arranged_things.stream.probe_with(&mut probe);
        arranged_things.trace
    });

    (input, probe, trace)
}