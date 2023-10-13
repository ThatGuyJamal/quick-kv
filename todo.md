# Todo

- [x] Make efficient File Read/Writer
- [x] Before database is initialized, read the current database file and load it into memory before accepting new data so we keep synced data within cache.
- [ ] Use rayon for parallelism
- [ ] Batching writes
- [ ] Tracks key TTLs (time to live) and delete them later (using thread channels)
- [ ] Add async client support

## Todo Notes

- Batching writes would enable use to save write lock time but temporarily waiting to write the data to the database file.
  This would be good if we have a lot of writes we need to do and we don't want to wait for them to all finish for other operations to happen.
  One problem is what happens if the program crashes before the writes are written to the database file? We could have a log file that we write to
  and then when the program starts up again we can read the log file and write the data to the database file. But then again, thats another file
  we have to read and write to. After the batch has added the new data to the file, it adds a batch-count in memory and when it hits a certain number
  we consume the current batch and write it to the database file. This way we can keep track of how many batches we need to fulfill. If the problem
  is shutdown and there is current data in the batch file, then on startup the program will attempt to write all the current batches to the database
  before it starts accepting new data. This will ensure the database is synced correctly.

  For the queue system in batching, we will use [VecDeque](https://doc.rust-lang.org/std/collections/struct.VecDeque.html)
