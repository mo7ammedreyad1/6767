use std::error::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use serde_json::json;
use monero::{Network, Address, PrivateKey, PublicKey};
use rand::RngCore;

const POOL_ADDRESS: &str = "pool.supportxmr.com:3322";

// دالة لتوليد محفظة XMR جديدة بالكامل بشكل صحيح (بدون أخطاء Compilation)
fn generate_new_wallet() -> String {
    println!("🔑 جاري توليد محفظة XMR جديدة...");
    let mut rng = rand::thread_rng();
    
    // 1. توليد مفتاح الإنفاق (Spend Key) السليم
    let spend_pub = loop {
        let mut bytes = [0u8; 32];
        rng.fill_bytes(&mut bytes);
        if let Ok(priv_key) = PrivateKey::from_slice(&bytes) {
            break PublicKey::from_private_key(&priv_key);
        }
    };

    // 2. توليد مفتاح المشاهدة (View Key) السليم
    let view_pub = loop {
        let mut bytes = [0u8; 32];
        rng.fill_bytes(&mut bytes);
        if let Ok(priv_key) = PrivateKey::from_slice(&bytes) {
            break PublicKey::from_private_key(&priv_key);
        }
    };
    
    // 3. بناء العنوان بالـ 3 مدخلات المطلوبة لحل خطأ E0061
    let address = Address::standard(Network::Mainnet, spend_pub, view_pub);
    let wallet_address = address.to_string();
    
    println!("✅ تم توليد المحفظة بنجاح!");
    println!("💰 عنوان المحفظة الجديد: {}", wallet_address);
    
    wallet_address
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("🚀 بدء تشغيل برنامج التعدين...");

    let wallet_address = generate_new_wallet();

    println!("🌐 جاري محاولة الاتصال بحوض التعدين: {}", POOL_ADDRESS);

    // معالجة خطأ الاتصال (اللي هيظهر على GitHub Actions بسبب حظر البورتات)
    let stream = match TcpStream::connect(POOL_ADDRESS).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("❌ فشل الاتصال بالحوض!");
            eprintln!("سبب الخطأ: {}", e);
            if e.kind() == std::io::ErrorKind::TimedOut {
                eprintln!("💡 تلميح: ده معناه إن السيرفر اللي مشغل عليه الكود (زي GitHub Actions) مانع الاتصال ببورتات التعدين عبر جدار الحماية (Firewall).");
            }
            return Err(e.into());
        }
    };

    let (reader, mut writer) = tokio::io::split(stream);
    let mut reader = BufReader::new(reader);

    let login_request = json!({
        "id": 1,
        "method": "login",
        "params": {
            "login": wallet_address,
            "pass": "x", 
            "agent": "Professional-Rust-Miner/1.1"
        }
    });

    let mut login_msg = login_request.to_string();
    login_msg.push('\n'); 
    
    if let Err(e) = writer.write_all(login_msg.as_bytes()).await {
        eprintln!("❌ فشل إرسال طلب تسجيل الدخول. السبب: {}", e);
        return Err(e.into());
    }
    println!("✅ تم إرسال طلب تسجيل الدخول للمحفظة الجديدة...");

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

fn handle_pool_response(response: &serde_json::Value) {
    if response["method"] == "job" {
        println!("🔥 استلمت مهمة تعدين جديدة!");
        let params = &response["params"];
        let job_id = params["job_id"].as_str().unwrap_or("مجهول");
        
        println!("🎯 رقم المهمة (Job ID): {}", job_id);
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
