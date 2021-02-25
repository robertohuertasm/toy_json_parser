# File reader

Super simple utility that will parse a file looking for a JSON in each line containing a `type` field.

It outputs a table with all the occurrences of a type and the amount of bytes per type.

A special type called `ERROR` will be created to account for all the invalid JSON lines the tool finds. See [Error](#errors) section below for more details.

**NOTE**: The code is commented to explain some of the decisions I made while doing this.

## Usage

```sh
# Use the -p flag to print results in a nicer table.
fr file_path [-p]

# example 
fr file2.txt 

# should exit something like this
# TYPE: A | TOTAL COUNT: 1 | TOTAL BYTES: 26
# TYPE: B | TOTAL COUNT: 3 | TOTAL BYTES: 122

# Took 134 microseconds

# example with pretty table
fr file2.txt -p

# should exit something like this
# +------+-------------+-------------+
# | TYPE | TOTAL COUNT | TOTAL BYTES |
# +------+-------------+-------------+
# | A    | 1           | 26          |
# +------+-------------+-------------+
# | B    | 3           | 122         |
# +------+-------------+-------------+
# Took 397 microseconds
```

## Errors

In case a line is not valid JSON, a new *TYPE* called **ERROR** will be shown in the table. The cli will output information about the kind of error in `stderr`.

The cli won't crash unless you use a non UTF-8 character encoding.

## Help

If you forget about the usage or you want to know more details about it just do this:

```sh
fr -h
```
