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

## PDF tools which I would need

- A functionality to visualise ALL the children of the catalog of a pdf file. An additional tool to visualise such tree for a whole filesystem, and in red evidentiated all the components which are not supported, and maybe in orange the components which are not particularly relevant, as `OpenAction`, which, if I well recall, tells only which action to perform when the PDF file is view, but it is not a fundamental information (often tells only the way you should visualise a pdf when you open it)

- A `pdf-toc-splitter`, i.e. a tool doing exactly the opposite of what I do in `pdfunite3`, separating a pdf into its components evidentiated in the Outline. But I wanna build my own: the actual `pdf-toc-splitter` embedds a pdf toc for each of the output files, and this is a problem for me. In fact often I want to modify a piece of my pdf making an annotation with Xournal++, and the procedure would be:
    1. separate the united pdf into its components according to the toc
    2. annotate the single component wiht Xournal++
    3. recompose the file with my tool
The last passage cannot be performed by my tool, if the components do have already an Outline. Besides, to program this tool should not be particularly nasty, because I would not need the option `level` of pdf-toc-splitter: I could simply split to the maximum level of the bookmarks. Notice also that if I would need an intermediate level I could always decompose completely the pdf into its atomic components, and then compose back only part of those with my original tool `pdfunite3`.