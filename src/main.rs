use std::net::UdpSocket;
use byteorder::{ByteOrder, BigEndian, WriteBytesExt};

#[derive(Debug)]
struct QueryHeader {
    identification: u16,
    flags: u16,
    question_count: u16,
    answer_count: u16,
    authority_records_count: u16,
    additional_records_count: u16
}

impl Default for QueryHeader {
    fn default() -> QueryHeader {
        QueryHeader {
            identification: 0,
            flags: 0,
            question_count: 0,
            answer_count: 0,
            authority_records_count: 0,
            additional_records_count: 0
        }
    }
}

#[derive(Debug)]
struct QueryQuestion {
    name: String,
    type_: u16,
    class: u16
}

impl Default for QueryQuestion {
    fn default() -> QueryQuestion {
        QueryQuestion {
            name: "".to_string(),
            type_: 0,
            class: 0
        }
    }
}

#[derive(Debug)]
struct QueryAnswer {
    name: String,
    type_: u16,
    class: u16,
    ttl: u32,
    rd_length: u16,
    r_data: u8 // Actually an array
}

impl Default for QueryAnswer {
    fn default() -> QueryAnswer {
        QueryAnswer {
            name: "".to_string(),
            type_: 0,
            class: 0,
            ttl: 0,
            rd_length: 0,
            r_data: 0
        }
    }
}

#[derive(Debug)]
struct QueryAuthority {}

impl Default for QueryAuthority {
    fn default() -> QueryAuthority {
        QueryAuthority {}
    }
}

#[derive(Debug)]
struct Query {
    header: QueryHeader,
    questions: Vec<QueryQuestion>,
    answer: QueryAnswer,
    authority: QueryAuthority
}

impl Default for Query {
    fn default() -> Query {
        Query {
            header: QueryHeader::default(),
            questions: Vec::new(),
            answer: QueryAnswer::default(),
            authority: QueryAuthority::default()
        }
    }
}


fn main() -> std::io::Result<()> {
    {
        println!("Socket create");
        let mut socket = UdpSocket::bind("0.0.0.0:53").expect("Unable to create socket");

        // Receives a single datagram message on the socket. If `buf` is too small to hold
        // the message, it will be cut off.
        let mut buf = [0; 128];
        println!("recv");
        let (amt, src) = socket.recv_from(&mut buf).expect("No data");
        println!("Got data: {:?}", &buf.to_vec());
//        println!("Test: {}", String::from_utf8(buf.to_vec()).expect("Not valid"));
//
        let mut query = Query::default();
        query.header.identification = BigEndian::read_u16(&buf[0..2]);// (buf[0] << 1 & buf[1]) as u16;
        query.header.flags = BigEndian::read_u16(&buf[2..4]);
        query.header.question_count = BigEndian::read_u16(&buf[4..6]);
        query.header.answer_count = BigEndian::read_u16(&buf[6..8]);
        query.header.authority_records_count = BigEndian::read_u16(&buf[8..10]);
        query.header.additional_records_count = BigEndian::read_u16(&buf[10..12]);

        for _question_index in 0..query.header.question_count {
            let mut question = QueryQuestion::default();
            // Read the label
            let mut label_pos = 12;
            let mut label = "".to_string();
            loop {
                let label_len = buf[label_pos];
                label_pos += 1;

                if label_len == 0 {
                    break;
                }

                for _label_char_index in 0..label_len {
                    label = format!("{}{}", label, buf[label_pos] as char);
                    label_pos += 1;
                    println!("label: {}", label);
                }
            }
            question.name = label;
            question.type_ = BigEndian::read_u16(&buf[label_pos..label_pos+2]);
            label_pos += 2;
            question.class = BigEndian::read_u16(&buf[label_pos..label_pos+2]);

            query.questions.push(question);
//            println!("Test: {}", String::from_utf8(labelBuf.to_vec()).expect("Not valid"));
        }

        println!("Got query: {:?}", query);




        let mut response = Query::default();
        response.header.identification = query.header.identification;
        response.header.answer_count = 1;
        response.header.question_count = 1;
        response.questions.push(QueryQuestion::default());
        response.questions[0].name = query.questions[0].name.clone();
        response.questions[0].type_ = query.questions[0].type_;
        response.questions[0].class = query.questions[0].class;
        //                         Q Op   A T R Ra Z  Rcd 
        response.header.flags = 0b_1_0000_0_0_0_1_000_0000;

        response.answer.name = "googlecom".to_string();
        response.answer.type_ = query.questions[0].type_;
        response.answer.class = query.questions[0].class;
        response.answer.ttl = 100;
        response.answer.rd_length = 4;



        let mut resp_bytes: Vec<u8> = Vec::new();
        resp_bytes.write_u16::<BigEndian>(response.header.identification).unwrap();
        resp_bytes.write_u16::<BigEndian>(response.header.flags).unwrap();
        resp_bytes.write_u16::<BigEndian>(response.header.question_count).unwrap();
        resp_bytes.write_u16::<BigEndian>(response.header.answer_count).unwrap();
        resp_bytes.write_u16::<BigEndian>(response.header.authority_records_count).unwrap();
        resp_bytes.write_u16::<BigEndian>(response.header.additional_records_count).unwrap();

        for _question_index in 0..response.header.question_count {
            let question = &response.questions[_question_index as usize];

            resp_bytes.write_u8(6 as u8).unwrap();
            for character in "google".chars() {
                resp_bytes.write_u8(character as u8).unwrap();
            }

            resp_bytes.write_u8(3 as u8).unwrap();
            for character in "com".chars() {
                resp_bytes.write_u8(character as u8).unwrap();
            }

            resp_bytes.write_u8(0).unwrap();

            resp_bytes.write_u16::<BigEndian>(question.type_).unwrap();
            resp_bytes.write_u16::<BigEndian>(question.class).unwrap();
        }

        for _answer_index in 0..response.header.answer_count {
            let answer = &response.answer;

            //resp_bytes.write_u8(0).unwrap();

            resp_bytes.write_u8(6 as u8).unwrap();
            for character in "google".chars() {
                resp_bytes.write_u8(character as u8).unwrap();
            }

            resp_bytes.write_u8(3 as u8).unwrap();
            for character in "com".chars() {
                resp_bytes.write_u8(character as u8).unwrap();
            }
            resp_bytes.write_u8(0).unwrap();

            resp_bytes.write_u16::<BigEndian>(answer.type_).unwrap();
            resp_bytes.write_u16::<BigEndian>(answer.class).unwrap();
            resp_bytes.write_u32::<BigEndian>(answer.ttl).unwrap();
            resp_bytes.write_u16::<BigEndian>(answer.rd_length).unwrap();

            resp_bytes.write_u8(69).unwrap();
            resp_bytes.write_u8(4).unwrap();
            resp_bytes.write_u8(20).unwrap();
            resp_bytes.write_u8(101).unwrap();
        }

        socket.send_to(&resp_bytes[..], &src)?;
        
    } // the socket is closed here
    Ok(())
}
