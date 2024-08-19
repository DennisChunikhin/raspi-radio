import numpy as np
from PIL import Image, ImageOps

import sys

if __name__=="__main__":
    # Help message
    if len(sys.argv)==1 or sys.argv[1] == "-h" or sys.argv[1] == "--help":
        print('''usage: python prep-image.py [options] image_path output_file_path
\toptions:
\t\t-h, --help\tThis message.
\t\t-d\t\tHow many distinct values to use. Must be >= 2. 2 by default.
\t\t-c\t\tCutoff value. 0 by default.
\t\t-w\t\tOutput image width. 100 by default.
\t\t-i\t\tInvert image. No value need be provided.
''')
        sys.exit(0)


    ## Parse command line arguments ##

    width=100
    num_divisions=1
    cutoff=0
    invert=False

    argc = len(sys.argv)
    i = 1
    while i<argc and sys.argv[i][0] == "-":
        if argc < i+2:
            sys.exit("Not enough arguments provided. Run python prep-image.py --help for usage.")

        if sys.argv[i] == "-d":
            num_divisions = int(sys.argv[i+1])-1
            if num_divisions < 1:
                sys.exit("-d must be at least 2")
        elif sys.argv[i] == "-c":
            cutoff = int(sys.argv[i+1])
        elif sys.argv[i] == "-w":
            width = int(sys.argv[i+1])
            if width < 1:
                sys.exit("-w must be at least 1")
        elif sys.argv[i] == "-i":
            invert = True
            i += 1
            continue
        else:
            sys.exit("Unsuported argument \"" + sys.argv[i] + "\". Run python prep-image.py --help for usage.")
        i += 2

    if argc < i+2:
        sys.exit("Not enough arguments provided. Run python prep-image.py --help for usage.")

    
    ## Image processing ##

    fout = sys.argv[i+1]

    img = Image.open(sys.argv[i])
    img = img.convert("L") # Convert to grayscale

    if invert:
        img = ImageOps.invert(img)

    # Resize image
    aspect = img.size[1] / img.size[0]
    img = img.resize((width, int(width*aspect)))

    # Pre-Processing
    img_data = np.asarray(img)
    
    img_data = img_data / np.max(img_data)
    img_data = (img_data * num_divisions).astype(int)
    img_data = np.maximum(img_data - cutoff, 0)

    wait_per_row = np.max(np.sum(img_data, axis=1))
    print(f"Wait steps per row: {wait_per_row}")
    

    ## Save formatted output ##

    data = []
    for row in img_data:
        wait_remaining = wait_per_row
        for i, px in enumerate(row):
            if px:
                data.append(i)
                data.append(px)
                wait_remaining -= px

        if wait_remaining != 0:
            data.append(-1)
            data.append(wait_remaining)

    np.savetxt(fout, [len(data)//2, wait_per_row] + data, fmt="%d")
