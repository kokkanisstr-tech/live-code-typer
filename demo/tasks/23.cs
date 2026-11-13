using System;
using System.Collections.Generic;

abstract class BankNotification
{
    public int NotificationId { get; set; }
    public string ClientName { get; set; }
    public string Date { get; set; }
    public bool IsRead { get; set; }

    public abstract void CreateMessage();
}

class DepositNotification : BankNotification
{
    public decimal DepositAmount { get; set; }

    public override void CreateMessage()
    {
        Console.WriteLine("№" + NotificationId + " " + ClientName + ": шотқа " + DepositAmount + " тг салынды (" + Date + ")");
    }
}

class WithdrawalNotification : BankNotification
{
    public decimal WithdrawalAmount { get; set; }

    public override void CreateMessage()
    {
        Console.WriteLine("№" + NotificationId + " " + ClientName + ": шоттан " + WithdrawalAmount + " тг алынды (" + Date + ")");
    }
}

class SecurityNotification : BankNotification
{
    public string EventDescription { get; set; }

    public override void CreateMessage()
    {
        Console.WriteLine("№" + NotificationId + " " + ClientName + ": қауіпсіздік - " + EventDescription + " (" + Date + ")");
    }
}

class Program
{
    static void Main()
    {
        List<BankNotification> notifications = new List<BankNotification>();
        notifications.Add(new DepositNotification { NotificationId = 1, ClientName = "Йусуф", Date = "11.11.2026", IsRead = false, DepositAmount = 150000 });
        notifications.Add(new DepositNotification { NotificationId = 2, ClientName = "Ерлан", Date = "11.11.2026", IsRead = true, DepositAmount = 50000 });
        notifications.Add(new WithdrawalNotification { NotificationId = 3, ClientName = "Йусуф", Date = "10.11.2026", IsRead = false, WithdrawalAmount = 20000 });
        notifications.Add(new WithdrawalNotification { NotificationId = 4, ClientName = "Дана", Date = "10.11.2026", IsRead = true, WithdrawalAmount = 7000 });
        notifications.Add(new SecurityNotification { NotificationId = 5, ClientName = "Йусуф", Date = "09.11.2026", IsRead = false, EventDescription = "жаңа құрылғыдан кіру" });
        notifications.Add(new SecurityNotification { NotificationId = 6, ClientName = "Ерлан", Date = "09.11.2026", IsRead = true, EventDescription = "құпиясөз өзгертілді" });

        int unread = 0;
        decimal depositSum = 0;
        foreach (BankNotification notification in notifications)
        {
            notification.CreateMessage();
            if (notification.IsRead == false)
            {
                unread = unread + 1;
                Console.WriteLine("  оқылмаған");
            }
            DepositNotification deposit = notification as DepositNotification;
            if (deposit != null)
            {
                depositSum = depositSum + deposit.DepositAmount;
                if (deposit.DepositAmount > 100000)
                {
                    Console.WriteLine("  салым 100 000 теңгеден жоғары");
                }
            }
        }
        Console.WriteLine("Оқылмаған хабарламалар: " + unread);
        Console.WriteLine("Салымдардың жалпы сомасы: " + depositSum + " тг");
    }
}
