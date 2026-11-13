using System;
using System.Collections.Generic;

abstract class PaymentMethod
{
    public int PaymentId { get; set; }
    public decimal Amount { get; set; }
    public bool IsSuccessful { get; set; }
    public decimal CommissionRate { get; set; }

    public abstract void ProcessPayment();

    protected void CheckAmount()
    {
        if (Amount > 0)
        {
            IsSuccessful = true;
        }
        else
        {
            IsSuccessful = false;
        }
    }
}

class BankCard : PaymentMethod
{
    public string CardType { get; set; }

    public override void ProcessPayment()
    {
        CommissionRate = 1;
        CheckAmount();
        Console.WriteLine("Карта: " + CardType + ", төлем " + PaymentId + ", сома " + Amount + ", комиссия 1%");
    }
}

class MobilePayment : PaymentMethod
{
    public string PhoneNumber { get; set; }

    public override void ProcessPayment()
    {
        CommissionRate = 0.5m;
        CheckAmount();
        Console.WriteLine("Мобильді төлем: " + PhoneNumber + ", төлем " + PaymentId + ", сома " + Amount + ", комиссия 0,5%");
    }
}

class BankTransfer : PaymentMethod
{
    public string BankName { get; set; }

    public override void ProcessPayment()
    {
        CommissionRate = 0.2m;
        CheckAmount();
        Console.WriteLine("Банктік аударым: " + BankName + ", төлем " + PaymentId + ", сома " + Amount + ", комиссия 0,2%");
    }
}

class Program
{
    static void Main()
    {
        List<PaymentMethod> payments = new List<PaymentMethod>();
        payments.Add(new BankCard { PaymentId = 1, Amount = 50000, CardType = "Visa" });
        payments.Add(new BankCard { PaymentId = 2, Amount = 0, CardType = "MasterCard" });
        payments.Add(new MobilePayment { PaymentId = 3, Amount = 15000, PhoneNumber = "87071234567" });
        payments.Add(new MobilePayment { PaymentId = 4, Amount = 25000, PhoneNumber = "87079876543" });
        payments.Add(new BankTransfer { PaymentId = 5, Amount = 100000, BankName = "Kaspi" });
        payments.Add(new BankTransfer { PaymentId = 6, Amount = 30000, BankName = "Halyk" });

        decimal successfulSum = 0;
        foreach (PaymentMethod payment in payments)
        {
            payment.ProcessPayment();
            decimal commission = payment.Amount * payment.CommissionRate / 100;
            Console.WriteLine("  комиссия: " + commission + " тг, нәтиже: " + (payment.IsSuccessful ? "сәтті" : "сәтсіз"));
            if (payment.IsSuccessful)
            {
                successfulSum = successfulSum + payment.Amount;
            }
        }
        Console.WriteLine("Сәтті төлемдердің жалпы сомасы: " + successfulSum + " тг");
    }
}
