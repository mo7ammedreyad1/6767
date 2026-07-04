use std::error::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use serde_json::json;
use monero::{Network, Address, KeyPair};
use rand_core::OsRng;

const POOL_ADDRESS: &str = "pool.supportxmr.com:3322";

// دالة لتوليد محفظة XMR جديدة بالكامل
fn generate_new_wallet() -> String {
    println!("🔑 جاري توليد محفظة XMR جديدة...");
    // توليد مفاتيح تشفير عشوائية
    let keypair = KeyPair::generate(&mut OsRng);
    // تحويل المفاتيح لعنوان شبكة Monero الأساسية (Mainnet)
    let address = Address::standard(Network::Mainnet, &keypair.public);
    let wallet_address = address.to_string();
    
    println!("✅ تم توليد المحفظة بنجاح!");
    println!("💰 عنوان المحفظة الجديد: {}", wallet_address);
    
    wallet_address
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("🚀 بدء تشغيل برنامج التعدين...");

    // 1. توليد المحفظة قبل أي حاجة
    let wallet_address = generate_new_wallet();

    println!("🌐 جاري محاولة الاتصال بحوض التعدين: {}", POOL_ADDRESS);

    // 2. معالجة احترافية لخطأ الاتصال
    let stream = match TcpStream::connect(POOL_ADDRESS).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("❌ فشل الاتصال بالحوض!");
            eprintln!("سباب الخطأ: {}", e);
            if e.kind() == std::io::ErrorKind::TimedOut {
                eprintln!("💡 تلميح: ده معناه إن السيرفر اللي مشغل عليه الكود (زي GitHub Actions) مانع الاتصال ببورتات التعدين.");
            }
            return Err(e.into());
        }
    };

    let (reader, mut writer) = tokio::io::split(stream);
    let mut reader = BufReader::new(reader);

    // 3. تجهيز رسالة الدخول بالمحفظة الجديدة
    let login_request = json!({
        "id": 1,
        "method": "login",
        "params": {
            "login": wallet_address,
            "pass": "x", 
            "agent": "Professional-Rust-Miner/1.0"
        }
    });

    let mut login_msg = login_request.to_string();
    login_msg.push('\n'); 
    
    if let Err(e) = writer.write_all(login_msg.as_bytes()).await {
        eprintln!("❌ فشل إرسال طلب تسجيل الدخول. السبب: {}", e);
        return Err(e.into());
    }
    println!("✅ تم إرسال طلب تسجيل الدخول للمحفظة الجديدة...");

    // 4. حلقة استقبال المهام
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => {
                println!("⚠️ السيرفر قفل الاتصال من عنده.");
                break;
            }
            Ok(_) => {
                if let Ok(response) = serde_json::from_str::<serde_json::Value>(&line) {
                    handle_pool_response(&response);
                } else {
                    eprintln!("⚠️ استلمت بيانات مشفرة أو غير مفهومة من السيرفر.");
                }
            }
            Err(e) => {
                eprintln!("❌ خطأ أثناء قراءة البيانات من السيرفر: {}", e);
                break;
            }
        }
    }

    Ok(())
}

// دالة منفصلة للتعامل مع ردود الحوض لترتيب الكود
fn handle_pool_response(response: &serde_json::Value) {
    if response["method"] == "job" {
        println!("🔥 استلمت مهمة تعدين جديدة!");
        let params = &response["params"];
        let job_id = params["job_id"].as_str().unwrap_or("مجهول");
        
        println!("🎯 رقم المهمة (Job ID): {}", job_id);
        println!("⚙️ جاري التعدين... (تتطلب دمج RandomX للعمل الفعلي)");
    } else if response["id"] == 1 {
         if response["error"].is_null() {
             println!("🔓 تم تسجيل الدخول بالحوض بنجاح بالمحفظة الجديدة!");
         } else {
             eprintln!("❌ الحوض رفض تسجيل الدخول. السبب: {:?}", response["error"]);
         }
    } else {
         println!("📩 إشعار من السيرفر: {:?}", response);
    }
}
