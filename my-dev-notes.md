# Development notes

## Extra

1. Add a feature to add a Page (or a series of pages) with the table of contents in the beginning.
2. write a small script to convert all your xournal files to pdf: https://github.com/xournalpp/xournalpp/issues/681, and then concatenate this alltogether. But this does not seem a good idea: my notes in xopp files do have an index in the beginnning, and this needs to be extrapolated and followed to create an outline following the personalised and written by hand content with `pdftoc.gen`
3. Make the generation of the outline an optional feature
4. flag to convert automatically the `xopp` files encountered in pdf and merge them in the process
5. If an error is returned because of a pdf file, the tool should collect all errors encountered, and print a the filesystem tree with the faulty entries colored in red and with the error message under them.
6. In case of success the app should also remind at the user that the tool to invert the process is `pdf-toc-split` (or how is it called)
7. Add support to treat the `Names` child of Catalog
8. Add support to ignore spcific features (non-supported catalog children) input by the user

## Executable names

- for `pdfunite-tree` it would be more optimal to be *maybe* `pdfunite3`, but this could be ambigous, even if shorter. Otherwise

- for commands create by me would be maybe good if they would be prefixed always with the pronoun `my-`. This would make much easier to modify them, and eventually erase them, without risking tomisunderstand them with builtin commands

## next ideas:

On a new branch (based on this one) develop this:
    - transform the `gen_rand_pdf` in a more general `pdf-util-by-me` or some other adapted name, where the generation of random pdfs is a subcommand (update: this idea is not good, there is already a tool called `pdfalyzer` which can output the whole PDF tree in colors, my tool would just be a toy version of it)
    - on this branch you might want to develop a couple of programs more to: 
        * visualise which children does catalog have. Example: does catalog have the subtree `/Names`? let us use the cmd line utility to print all those children there
        * visualise specific subtree of the PDF document as the command tree does. For example if we would like to visualise the subtree `Outlines` we would like to see the subtree of all objects `Outline` with dereferenced locations, title etc.. This might be interesting for the key `Pages` or similar.

    - the programs developed should possibly have most  of the running capabilities in `utils.rs`, and use in the bin only the functinos defined in `lib.rs`

    With these instrument in place I should be able to tackle my most important challenges: 
    - debug my program (understanding what is wrong in the way I build the outline)
    - Immediately understand which children does catalog have in my notes, or most of them, so to learn which are the most commmon children that my program should be able to treat (as for examplee `/Names`)

    Watching my notes of SDE, I see that many pdf docs produced do implement the tree `/Names` and `OpenAction`. I maybe also would like a flag to ignore the child `Metadata`. I would like, in case of failure, that the tool still continues to say me which files and in which location are giving me problems, and what type of problem (which feature is problematic). Idea, implement a trait Tree (or search online if somebody has done it) and let the tool print the tree of the folder, with in red the locations giving problmes, and under each location the problem it encountered (presence of some broken feature? not a pdf file?)


## Tests

How to implement the tests? I need a way to validate the PDFs and also to check what the outline is, but automatically. In which way to do it? I found a series of instrumens and candidates.

1. Validate the pdf:
    * pdfinfo
    * qpdf
    * pdftotext

2. Extract the Outline
    * pdftk
    * pdfoutliner

Any of these, if added, must be declared as a requirement for the tests on the README.md