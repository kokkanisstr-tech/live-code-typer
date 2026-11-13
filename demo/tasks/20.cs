using System;
using System.Collections.Generic;

abstract class Message
{
    public string Recipient { get; set; }
    public string Text { get; set; }
    public string SendDate { get; set; }
    public bool IsSent { get; set; }

    public abstract void Send();
}

class EmailMessage : Message
{
    public string EmailAddress { get; set; }

    public override void Send()
    {
        if (EmailAddress == null || EmailAddress == "")
        {
            IsSent = false;
            return;
        }
        IsSent = true;
        Console.WriteLine("Email жіберілді: " + Recipient + " <" + EmailAddress + "> - " + Text);
    }
}

class SmsMessage : Message
{
    public string PhoneNumber { get; set; }

    public override void Send()
    {
        if (PhoneNumber == null || PhoneNumber == "")
        {
            IsSent = false;
            return;
        }
        IsSent = true;
        Console.WriteLine("SMS жіберілді: " + Recipient + " <" + PhoneNumber + "> - " + Text);
    }
}

class PushMessage : Message
{
    public string ApplicationName { get; set; }

    public override void Send()
    {
        if (ApplicationName == null || ApplicationName == "")
        {
            IsSent = false;
            return;
        }
        IsSent = true;
        Console.WriteLine("Push жіберілді: " + Recipient + " <" + ApplicationName + "> - " + Text);
    }
}

class Program
{
    static void Main()
    {
        List<Message> messages = new List<Message>();
        messages.Add(new EmailMessage { Recipient = "Йусуф", Text = "Сәлем", SendDate = "11.11.2026", EmailAddress = "yusuf@mail.kz" });
        messages.Add(new EmailMessage { Recipient = "Ерлан", Text = "Кездесу", SendDate = "11.11.2026", EmailAddress = "" });
        messages.Add(new SmsMessage { Recipient = "Ана", Text = "Келдім", SendDate = "11.11.2026", PhoneNumber = "87071112233" });
        messages.Add(new SmsMessage { Recipient = "Аға", Text = "Хабарлас", SendDate = "11.11.2026", PhoneNumber = "" });
        messages.Add(new PushMessage { Recipient = "Йусуф", Text = "Жаңа тапсырма", SendDate = "11.11.2026", ApplicationName = "SmartNation" });
        messages.Add(new PushMessage { Recipient = "Йусуф", Text = "Еске салу", SendDate = "11.11.2026", ApplicationName = "Календарь" });

        int sent = 0;
        foreach (Message message in messages)
        {
            message.Send();
            if (message.IsSent)
            {
                sent = sent + 1;
            }
            else
            {
                Console.WriteLine("Жіберілмеді: " + message.Recipient + " - мекенжай немесе желі бос");
            }
        }
        Console.WriteLine("Жіберілген хабарламалар: " + sent);
        Console.WriteLine("Жіберілмегендер: " + (messages.Count - sent));
    }
}
