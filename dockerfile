FROM jbarlow83/ocrmypdf
WORKDIR ./app
COPY dist/ .
ENTRYPOINT ["./ocrpdf"]