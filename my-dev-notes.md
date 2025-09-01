# Development notes

To achieve my goals I have two options:

1. Writing a program which calls the right command line tools automatically, with the correct arguments etc... In this sense my program would only be a shell automatization, and thus every programming language would suit the purpose. If I choose this road I could use the occasion to learn a bit of `bash` coding, but also python, julia and rust would be perfect.
2. Write a program which does everything on its own framework, without calling external tools. This would be the most elegant and efficient solution. Here the languages do influence the outcome.

## outline of solution 1 (bash?)

I scan the rootfolder and all the ones below. I check that I do not go over 4 levels deep. I validate the entries:
- I check that all entries are either folders or pdf files.
- I call `pdfinfo` on each file and check that at the voices `Custom Metadata` and `metadata stream` there is the option `No`

I then register the number of pages of each pdf file and the order of the pdfs (alphabetical?). I build consequently the input file for the ToC to be made with `pdftocgen`

I call then the command line tool `pdfunite` in such a way that all pdfs are attached. I then create the ToC with the `pdftocgen`.

If I want also the initial index I can write a markdown file with such index, render it with `pandoc` and then append it to the beginning of the file and move all the number of pages accordingly in the ToC.

Notice that this solution is complete, meaning that all the plan is outlined.

## solution with lopdf and Rust

1. Parse the cli option (flag to add the index page(s) in the beginning, flag to convert automatically the `xopp` files encountered)
2. Pass the cli to a run function
3. the run function creates a main doc and initialises it. Then passes the main_doc, the root folder and the options to a function called 'merge_from_directory'. This is meant to act as the recersive function on tree nodes, on which we need a condition to halt recursion. 

Function which acts on nodes:
    If encounters a files calls the function to be called on leaves: merge_doc (main_doc, parent_bookmark, file (as path or as name?), flag for xopp files)
    if encounters a directory or a symlink: it calls itself if an halt condition of max_tree_depth is respected, otherwise it yields error

Function which acts on leafs:
    Checks that the file is `.pdf` marked and tries to open it

## Extra

1. Add a feature to add a Page (or a series of pages) with the table of contents in the beginning.
2. write a small script to convert all your xournal files to pdf: https://github.com/xournalpp/xournalpp/issues/681, and then concatenate this alltogether. But this does not seem a good idea: my notes in xopp files do have an index in the beginnning, and this needs to be extrapolated and followed to create an outline following the personalised and written by hand content with `pdftoc.gen`
3. Make the generation of the outline an optional feature

## Executable names

- for `pdfunite-tree` it would be more optimal to be *maybe* `pdfunite3`, but this could be ambigous, even if shorter. Otherwise

- `gen_rand_pdf` is not really useful to anybody else and it is obvious that the doc generated is random, thus I could call it simply `gen-pdf`

## next ideas:

On a new branch (based on this one) develop this:
    - transform the `gen_rand_pdf` in a more general `pdf-util-by-me` or some other adapted name, where the generation of random pdfs is a subcommand
    - on this branch you might want to develop a couple of programs more to: 
        * visualise which children does catalog have. Example: does catalog have the subtree `/Names`? let us use the cmd line utility to print all those children there
        * visualise specific subtree of the PDF document as the command tree does. For example if we would like to visualise the subtree `Outlines` we would like to see the subtree of all objects `Outline` with dereferenced locations, title etc.. This might be interesting for the key `Pages` or similar.

    - the programs developed should possibly have most  of the running capabilities in `utils.rs`, and use in the bin only the functinos defined in `lib.rs`

    With these instrument in place I should be able to tackle my most important challenges: 
    - debug my program (understanding what is wrong in the way I build the outline)
    - Immediately understand which children does catalog have in my notes, or most of them, so to learn which are the most commmon children that my program should be able to treat (as for examplee `/Names`)
