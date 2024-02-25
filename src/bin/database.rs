use snapcase::demo::database::PurchaseDatabase;


fn main() {
    let db = PurchaseDatabase::new();
    let purchases = db.purchases(40058);
    eprintln!("{:?}", purchases);

    db.alcohol_purchases(40058, |(b, i)| eprintln!("{b} {i}"));
}