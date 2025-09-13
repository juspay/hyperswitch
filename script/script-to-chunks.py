import os

def split_csv_by_size(input_file, max_size_mb=100):
    part_num = 1
    out_file = open(f"part_{part_num}.csv", "w", encoding="utf-8")

    with open(input_file, "r", encoding="utf-8") as f:
        header = f.readline()
        out_file.write(header)
        size = out_file.tell()

        for line in f:
            if size > max_size_mb * 1024 * 1024:
                out_file.close()
                part_num += 1
                out_file = open(f"part_{part_num}.csv", "w", encoding="utf-8")
                out_file.write(header)
                size = out_file.tell()
            out_file.write(line)
            size = out_file.tell()

    out_file.close()

# Run with 500 MB chunks
split_csv_by_size("copy.csv", max_size_mb=100)
