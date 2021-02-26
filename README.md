# File reader

Super simple utility that will parse a file looking for a JSON in each line containing a `type` field.

It outputs a table with all the occurrences of a type and the amount of bytes per type.

A special type called `ERROR` will be created to account for all the invalid JSON lines the tool finds. See [Error](#errors) section below for more details.

**IMPORTANT**: Check out the [Rationale](#rationale) section to know some details about the implementation decisions.

## Usage

The tools comes with two different parsing strategies:

- **Naive** (default): ideal for small files as it will usually be faster.
- **Chunk reading**: ideal for big files as it will try to parse the file by chunks and use different threads.

```sh
# Use the -p flag to print results in a nicer table.
fr file_path [-p]

# example 
fr file2.txt

# should exit something like this
# TYPE: A | TOTAL COUNT: 1 | TOTAL BYTES: 26
# TYPE: B | TOTAL COUNT: 3 | TOTAL BYTES: 133
# TYPE: ERROR | TOTAL COUNT: 2 | TOTAL BYTES: 2

# Took 131 microseconds

# example with pretty table
fr file2.txt -p

# should exit something like this
# +-------+-------------+-------------+
# | TYPE  | TOTAL COUNT | TOTAL BYTES |
# +-------+-------------+-------------+
# | B     | 3           | 133         |
# +-------+-------------+-------------+
# | ERROR | 2           | 2           |
# +-------+-------------+-------------+
# | A     | 1           | 26          |
# +-------+-------------+-------------+
# Took 397 microseconds
```

If you need to parse big files then you should use it like this:

```sh
fr file_big.txt -p -c 

# +------+-------------+-------------+
# | TYPE | TOTAL COUNT | TOTAL BYTES |
# +------+-------------+-------------+
# | C    | 68796       | 3233412     |
# +------+-------------+-------------+
# | D    | 163800      | 7698600     |
# +------+-------------+-------------+
# | B    | 7488        | 357084      |
# +------+-------------+-------------+
# | A    | 98514       | 4630158     |
# +------+-------------+-------------+
# Took 67566 microseconds

# You can fine tune the chunk-size (1_000_000 by default):
fr file_big.txt -p -c --chunk-size 1500000
# +------+-------------+-------------+
# | TYPE | TOTAL COUNT | TOTAL BYTES |
# +------+-------------+-------------+
# | B    | 7488        | 357084      |
# +------+-------------+-------------+
# | D    | 163800      | 7698600     |
# +------+-------------+-------------+
# | C    | 68796       | 3233412     |
# +------+-------------+-------------+
# | A    | 98514       | 4630158     |
# +------+-------------+-------------+
# Took 66703 microseconds
```

## Errors

In case a line is not valid JSON, a new *TYPE* called **ERROR** will be shown in the table.

The cli won't crash unless you use a non UTF-8 character encoding.

## Help

If you forget about the usage or you want to know more details about it just do this:

```sh
fr -h
```

## Rationale

I ended up providing two different approaches that can be selected via the cli.

My first implementation was the naive one, which worked very well for small files but could not be performant in case we had to deal with big files.

Then I thought about using [rayon](https://docs.rs/rayon/1.5.0/rayon/), parallelizing the [BufRead lines iterator](https://doc.rust-lang.org/std/io/trait.BufRead.html#method.lines) but I found it was significantly slower, probably because of synchronization issues with the mutex that I wasn't able to overcome.

I also tried to read the file in chunks and use [rayon](https://docs.rs/rayon/1.5.0/rayon/) to spawn some jobs to do the parsing but I got a pretty similar result.

That led me to another approach in which I read the file in chunks and spawned some [threads](https://doc.rust-lang.org/std/thread/index.html) to do the computation while using a [channel](https://doc.rust-lang.org/std/sync/mpsc/fn.channel.html) to get all the results and digest them afterwards. This approach seemed to be faster for bigger files.

I didn't use [tokio](https://docs.rs/tokio/1.2.0/tokio/index.html) and [futures](https://doc.rust-lang.org/std/future/index.html) and dediced to go with plain [threads](https://doc.rust-lang.org/std/thread/index.html) because of time constraints.

Regarding `JSON` deserialization I went for [serde](https://docs.rs/serde/1.0.123/serde/) and [serde_json](https://docs.rs/serde_json/1.0.63/serde_json/) which are always a safe bet on this matter.

I added some test while developing this. Although they're not covering all the cases they were pretty useful to be sure that I was supporting at least the main cases.

Finally, I decided to use [structopt](https://docs.rs/structopt/0.3.21/structopt/), which is my go-to crate when building `CLIs`. I'm pretty acquanited with it and I like how much easier it makes creating `CLIs`.

Thanks for the time you spent reviewing this! Glad to hear some suggestions, ideas and improvements! Still so much to learn ;P
