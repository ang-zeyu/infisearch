use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::collections::VecDeque;
use std::sync::Arc;

use dashmap::DashMap;

use super::{PostingsStreamDecoder, TermDocsForMerge};
use crate::MainToWorkerMessage;
use crate::Receiver;
use crate::Sender;

pub static POSTINGS_STREAM_BUFFER_SIZE: usize = 3000;

pub static POSTINGS_STREAM_INITIAL_READ: usize = 2000;

static POSTINGS_STREAM_READER_ADVANCE_READ_THRESHOLD: usize = 1000;

pub struct PostingsStream {
    idx: u32,
    is_empty: bool,
    is_reader_decoding: bool,
    curr_term: TermDocsForMerge,
    term_buffer: VecDeque<TermDocsForMerge>,
}

// Order by term, then block number
impl Eq for PostingsStream {}

impl PartialEq for PostingsStream {
    fn eq(&self, other: &Self) -> bool {
        self.curr_term.term == other.curr_term.term && self.idx == other.idx
    }
}

impl Ord for PostingsStream {
    fn cmp(&self, other: &Self) -> Ordering {
        match other.curr_term.term.cmp(&self.curr_term.term) {
            Ordering::Equal => other.idx.cmp(&self.idx),
            Ordering::Greater => Ordering::Greater,
            Ordering::Less => Ordering::Less,
        }
    }
}

impl PartialOrd for PostingsStream {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match other.curr_term.term.cmp(&self.curr_term.term) {
            Ordering::Equal => Some(other.idx.cmp(&self.idx)),
            Ordering::Greater => Some(Ordering::Greater),
            Ordering::Less => Some(Ordering::Less),
        }
    }
}

impl PostingsStream {
    pub fn initialise_postings_streams(
        first_block: u32,
        last_block: u32,
        postings_stream_heap: &mut BinaryHeap<PostingsStream>,
        postings_stream_decoders: &Arc<DashMap<u32, PostingsStreamDecoder>>,
        tx_main: &Sender<MainToWorkerMessage>,
        blocking_sndr: &Sender<()>,
        blocking_rcvr: &Receiver<()>,
    ) {
        for idx in first_block..(last_block + 1) {
            let postings_stream = Self::create_postings_stream(
                idx, postings_stream_decoders, tx_main, blocking_sndr, blocking_rcvr
            );
            if postings_stream.curr_term.term.len() != 0 {
                postings_stream_heap.push(postings_stream);
            }
        }
    }

    fn create_postings_stream(
        postings_stream_number: u32,
        postings_stream_decoders: &Arc<DashMap<u32, PostingsStreamDecoder>>,
        tx_main: &Sender<MainToWorkerMessage>,
        blocking_sndr: &Sender<()>,
        blocking_rcvr: &Receiver<()>,
    ) -> Self {
        let mut postings_stream = PostingsStream {
            idx: postings_stream_number,
            is_empty: false,
            is_reader_decoding: true,
            curr_term: Default::default(),
            term_buffer: VecDeque::with_capacity(POSTINGS_STREAM_BUFFER_SIZE), // transfer ownership of future term buffer to the main postings stream
        };
        postings_stream.get_term(&postings_stream_decoders, tx_main, blocking_sndr, blocking_rcvr, false);
        postings_stream
    }

    fn get_term(
        &mut self,
        postings_stream_decoders: &Arc<DashMap<u32, PostingsStreamDecoder>>,
        tx_main: &Sender<MainToWorkerMessage>,
        blocking_sndr: &Sender<()>,
        blocking_rcvr: &Receiver<()>,
        _do_print_blocked_msg: bool,
    ) {
        if self.term_buffer.is_empty() {
            let mut lock = postings_stream_decoders.get_mut(&self.idx).unwrap();
            let lock_value_mut = lock.value_mut();
            match lock_value_mut {
                PostingsStreamDecoder::Reader(postings_stream_reader) => {
                    std::mem::swap(&mut postings_stream_reader.future_term_buffer, &mut self.term_buffer);
                }
                PostingsStreamDecoder::None => {
                    #[cfg(debug_assertions)]
                    if _do_print_blocked_msg {
                        println!("Blocked! Ouch! Consider increasing the decode buffer size...");
                    }

                    // Set to notifier
                    *lock_value_mut = PostingsStreamDecoder::Notifier(blocking_sndr.clone());

                    // Deadlock otherwise - worker will never be able to acquire postings_stream_readers_vec
                    drop(lock);

                    // Wait for worker to finish decoding...
                    blocking_rcvr.recv().unwrap();

                    // Done! Reacquire lock
                    match postings_stream_decoders.get_mut(&self.idx).unwrap().value_mut() {
                        PostingsStreamDecoder::Reader(postings_stream_reader) => {
                            std::mem::swap(
                                &mut postings_stream_reader.future_term_buffer,
                                &mut self.term_buffer,
                            );
                        }
                        _ => panic!("Unexpected state @get_term blocking branch"),
                    }
                }
                _ => panic!("Unexpected state @get_term notifier"),
            }
            self.is_reader_decoding = false;
        } else if !self.is_reader_decoding
            && self.term_buffer.len() < POSTINGS_STREAM_READER_ADVANCE_READ_THRESHOLD
        {
            // Request for an in-advance worker decode...

            match std::mem::replace(
                postings_stream_decoders.get_mut(&self.idx).unwrap().value_mut(),
                PostingsStreamDecoder::None,
            ) {
                PostingsStreamDecoder::Reader(postings_stream_reader) => {
                    postings_stream_reader.read_next_batch(POSTINGS_STREAM_BUFFER_SIZE, tx_main, Arc::clone(postings_stream_decoders));
                    self.is_reader_decoding = true;
                }
                _ => panic!("Unexpected state @get_term"),
            }
        }

        if let Some(term_termdocs) = self.term_buffer.pop_front() {
            // Transfer first term of term_buffer into curr_term and curr_term_docs
            self.curr_term = term_termdocs;
        } else {
            // Mark as exhausted otherwise
            self.is_empty = true;
        }
    }

    #[inline(always)]
    pub fn aggregate_block_terms(
        curr_combined_term_docs: &mut Vec<TermDocsForMerge>,
        postings_streams: &mut BinaryHeap<PostingsStream>,
        postings_stream_decoders: &Arc<DashMap<u32, PostingsStreamDecoder>>,
        tx_main: &Sender<MainToWorkerMessage>,
        blocking_sndr: &Sender<()>,
        blocking_rcvr: &Receiver<()>,
    ) -> (String, u32, f32) {
        curr_combined_term_docs.clear();

        let mut postings_stream = postings_streams.pop().unwrap();

        let mut doc_freq = postings_stream.curr_term.doc_freq;

        let curr_term = std::mem::take(&mut postings_stream.curr_term.term);
        let mut curr_term_max_score = postings_stream.curr_term.max_doc_term_score;
        curr_combined_term_docs.push(std::mem::take(&mut postings_stream.curr_term));

        postings_stream.get_term(&postings_stream_decoders, tx_main, blocking_sndr, blocking_rcvr, true);
        if !postings_stream.is_empty {
            postings_streams.push(postings_stream);
        }

        // Aggregate same terms from different blocks...
        while !postings_streams.is_empty() && postings_streams.peek().unwrap().curr_term.term == curr_term {
            postings_stream = postings_streams.pop().unwrap();

            doc_freq += postings_stream.curr_term.doc_freq;

            if postings_stream.curr_term.max_doc_term_score > curr_term_max_score {
                curr_term_max_score = postings_stream.curr_term.max_doc_term_score;
            }
            curr_combined_term_docs.push(std::mem::take(&mut postings_stream.curr_term));

            postings_stream.get_term(&postings_stream_decoders, tx_main, blocking_sndr, blocking_rcvr, true);
            if !postings_stream.is_empty {
                postings_streams.push(postings_stream);
            }
        }

        (curr_term, doc_freq, curr_term_max_score)
    }
}
