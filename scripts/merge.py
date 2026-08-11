import argparse
import os
import sys

def merge_files(file1, file2, file3, output_file):
    # Define the target offsets for each file
    # Format: (File Path, Offset Address)
    files_config = [
        (file1, 0x0),
        (file2, 0x80000),
        (file3, 0x200000)
    ]

    try:
        with open(output_file, 'wb') as f_out:
            print(f"Creating merged file: {output_file}")

            for f_path, offset in files_config:
                # 1. Check if the file exists
                if not os.path.exists(f_path):
                    print(f"Error: Input file not found: {f_path}")
                    sys.exit(1)

                # 2. Check for overlaps
                # Get the current position of the write cursor
                current_pos = f_out.tell()
                
                if current_pos > offset:
                    print(f"Error: Overlap detected!")
                    print(f"Previous file ended at {hex(current_pos)}, which is past the next offset {hex(offset)}.")
                    sys.exit(1)

                # 3. Fill the gap with 0x00
                # We calculate the gap size and write zeros explicitly.
                # (Alternatively, f_out.seek(offset) would implicitly fill gaps with zeros 
                # on most systems, but explicit writing guarantees it).
                gap_size = offset - current_pos
                if gap_size > 0:
                    print(f"Filling gap of {gap_size} bytes with 0x00...")
                    f_out.write(b'\x00' * gap_size)

                # 4. Write the file content
                print(f"Writing {f_path} at offset {hex(offset)}...")
                with open(f_path, 'rb') as f_in:
                    data = f_in.read()
                    f_out.write(data)

            print("\nSuccess! Files combined.")
            print(f"Total size: {f_out.tell()} bytes ({hex(f_out.tell())})")

    except IOError as e:
        print(f"File I/O Error: {e}")
        sys.exit(1)

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Merge three binary files at specific offsets.")
    
    # Define arguments
    parser.add_argument("file1", help="Path to the first file (Offset 0x0)")
    parser.add_argument("file2", help="Path to the second file (Offset 0x80000)")
    parser.add_argument("file3", help="Path to the third file (Offset 0x200000)")
    parser.add_argument("-o", "--output", default="xiangshan_software.bin", help="Path for the output file (default: xiangshan_software.bin)")

    args = parser.parse_args()

    merge_files(args.file1, args.file2, args.file3, args.output)
