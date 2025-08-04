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

## which language and library should I use in case I adopt solution 2?

### if Rust

Rust is ofc one of the choices. The problem is that the libraries in Rust are not completely exaustive. The most complete one is `lopdf`, but it is absolutely not documented and more of a general rust transposition of pdfs logic in Rust, rather than a tool which allows to work efficientely with such files. Otherwise there is `pdfium-render`, but it is not clear to me if the features of it do allow for a manipulation of the ToC of a file.

