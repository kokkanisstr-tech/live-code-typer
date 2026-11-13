using System;
using System.Collections.Generic;

abstract class Document
{
    public string FileName { get; set; }
    public double FileSize { get; set; }
    public string Author { get; set; }
    public int CreationYear { get; set; }

    public abstract void Open();
}

class TextDocument : Document
{
    public int WordCount { get; set; }

    public override void Open()
    {
        Console.WriteLine("Мәтіндік құжат ашылды: " + FileName + ", сөз саны " + WordCount);
    }
}

class PdfDocument : Document
{
    public int PageCount { get; set; }

    public override void Open()
    {
        Console.WriteLine("PDF құжат ашылды: " + FileName + ", бет саны " + PageCount);
    }
}

class WordDocument : Document
{
    public bool HasImages { get; set; }

    public override void Open()
    {
        Console.WriteLine("Word құжат ашылды: " + FileName + ", суреттер: " + (HasImages ? "бар" : "жоқ"));
    }
}

class Program
{
    static void Main()
    {
        List<Document> documents = new List<Document>();
        documents.Add(new TextDocument { FileName = "esep.txt", FileSize = 2.5, Author = "Йусуф", CreationYear = 2026, WordCount = 1200 });
        documents.Add(new TextDocument { FileName = "lekciya.txt", FileSize = 1.2, Author = "Арай", CreationYear = 2025, WordCount = 800 });
        documents.Add(new PdfDocument { FileName = "praktika.pdf", FileSize = 15.5, Author = "Йусуф", CreationYear = 2026, PageCount = 25 });
        documents.Add(new PdfDocument { FileName = "kitap.pdf", FileSize = 8.0, Author = "Ауезов", CreationYear = 2020, PageCount = 300 });
        documents.Add(new WordDocument { FileName = "referat.docx", FileSize = 12.3, Author = "Йусуф", CreationYear = 2026, HasImages = true });
        documents.Add(new WordDocument { FileName = "test.docx", FileSize = 0.8, Author = "Арай", CreationYear = 2025, HasImages = false });

        foreach (Document document in documents)
        {
            document.Open();
        }

        Console.WriteLine("--- Көлемі 10 МБ-тан асатын құжаттар ---");
        double total = 0;
        foreach (Document document in documents)
        {
            total = total + document.FileSize;
            if (document.FileSize > 10)
            {
                Console.WriteLine(document.FileName + " - " + document.FileSize + " МБ");
            }
        }
        Console.WriteLine("Жалпы көлем: " + total + " МБ");
    }
}
