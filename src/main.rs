use std::net::{UdpSocket, SocketAddr, IpAddr, Ipv4Addr};
use byteorder::{ByteOrder, BigEndian, WriteBytesExt};

#[derive(Debug, Clone)]
struct QueryToken {
    data: String,
}

impl Default for QueryToken {
    fn default() -> QueryToken {
        QueryToken {
            data: "".to_string()
        }
    }
}

impl QueryToken {
    fn new(data: &str) -> QueryToken {
        QueryToken {
            data: data.to_string()
        }
    }

    fn write(&self, buf: &mut Vec<u8>) {
        buf.write_u8(self.data.len() as u8).unwrap();
        for token_char in self.data.chars() {
            buf.write_u8(token_char as u8).unwrap();
        }
    }
}

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

impl QueryHeader {
    fn write(&self, buf: &mut Vec<u8>) {
        buf.write_u16::<BigEndian>(self.identification).unwrap();
        buf.write_u16::<BigEndian>(self.flags).unwrap();
        buf.write_u16::<BigEndian>(self.question_count).unwrap();
        buf.write_u16::<BigEndian>(self.answer_count).unwrap();
        buf.write_u16::<BigEndian>(self.authority_records_count).unwrap();
        buf.write_u16::<BigEndian>(self.additional_records_count).unwrap();
    }
}

#[derive(Debug, Clone)]
struct QueryQuestion {
    name: Vec<QueryToken>,
    type_: u16,
    class: u16
}

impl Default for QueryQuestion {
    fn default() -> QueryQuestion {
        QueryQuestion {
            name: Vec::new(),
            type_: 0,
            class: 0
        }
    }
}

impl QueryQuestion {
    fn write(&self, buf: &mut Vec<u8>) {
        for name_token in &self.name {
            name_token.write(buf);
        }

        buf.write_u8(0).unwrap();

        buf.write_u16::<BigEndian>(self.type_).unwrap();
        buf.write_u16::<BigEndian>(self.class).unwrap();
    }
}

#[derive(Debug)]
struct QueryAnswer {
    name: Vec<QueryToken>,
    type_: u16,
    class: u16,
    ttl: u32,
    rd_length: u16,
    r_data: Vec<u8>
}

impl Default for QueryAnswer {
    fn default() -> QueryAnswer {
        QueryAnswer {
            name: Vec::new(),
            type_: 0,
            class: 0,
            ttl: 0,
            rd_length: 0,
            r_data: Vec::new()
        }
    }
}

impl QueryAnswer {
    fn write(&self, buf: &mut Vec<u8>) {
        for name_token in &self.name {
            name_token.write(buf);
        }

        // Send a null terminator
        buf.write_u8(0).unwrap();

        buf.write_u16::<BigEndian>(self.type_).unwrap();
        buf.write_u16::<BigEndian>(self.class).unwrap();
        buf.write_u32::<BigEndian>(self.ttl).unwrap();
        buf.write_u16::<BigEndian>(self.rd_length).unwrap();

        for octet in &self.r_data {
            buf.write_u8(*octet).unwrap();
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

impl QueryAuthority {
    fn write(&self, buf: &mut Vec<u8>) {}
}

#[derive(Debug)]
struct Query {
    header: QueryHeader,
    questions: Vec<QueryQuestion>,
    answers: Vec<QueryAnswer>,
    authorities: Vec<QueryAuthority>,

    requester: SocketAddr
}

impl Default for Query {
    fn default() -> Query {
        Query {
            header: QueryHeader::default(),
            questions: Vec::new(),
            answers: Vec::new(),
            authorities: Vec::new(),
            requester: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080)
        }
    }
}

impl Query {
    fn new(src: SocketAddr) -> Query {
        Query {
            header: QueryHeader::default(),
            questions: Vec::new(),
            answers: Vec::new(),
            authorities: Vec::new(),
            requester: src
        }
    }

    fn write(&self, buf: &mut Vec<u8>) {
        self.header.write(buf);
        
        for question in &self.questions {
            question.write(buf);
        }

        for answer in &self.answers {
            answer.write(buf);
        }

        for authority in &self.authorities {
            authority.write(buf);
        }
    }
}

#[derive(Debug)]
enum Record {
    A {addr: [u8; 4]},
    Invalid
}

#[derive(Debug)]
struct CacheNode {
    label: QueryToken,
    //record: Record,
    data: Vec<u8>,
    children: Vec<CacheNode>
}

impl CacheNode {
    fn new(label: QueryToken, data: Vec<u8>) -> CacheNode {
        CacheNode {
            label: label,
            data,
            children: Vec::new()
        }
    }

    fn has_label(&self, needle: &QueryToken) -> bool {
        //println!("Checking if {:?} has {:?}",self, needle); 
        for child in &self.children {
            if child.label.data == needle.data {
                return true;
            }
        }
        return false;
    }

    fn search_by_label(&mut self, needle: &QueryToken) -> Option<&mut CacheNode> {
        //println!("Searching for {:?} in {:?}", needle, self);
        for child in &mut self.children {
            if child.label.data == needle.data {
                return Some(child);
            }
        }

        return None;
    }

    fn search_by_label_stream(&mut self, needle: &Vec<&QueryToken>) -> &mut CacheNode {
        //println!("Searching {:?} for {:?}", self, needle);
        let mut index = 0;
        if !self.has_label(&needle[0]) {
            //println!("Dont know label: {:?}", &needle[0]);
            return self;
        }
        let mut current_root = self.search_by_label(&needle[index]).unwrap();

        loop {
            // Are we done?
            if current_root.label.data == needle.last().unwrap().data {
                return current_root;
            }
            // Does the current root have the next bit
            index += 1;
            if current_root.has_label(&needle[index]) {
                current_root = current_root.search_by_label(&needle[index]).unwrap();
            } else {
                return current_root;
            }
        }
    }

    fn insert(&mut self, name: &QueryToken, data: Vec<u8>) -> &mut CacheNode {
        let new_node = CacheNode::new(name.clone(), data);
        self.children.push(new_node);
        return self.children.last_mut().unwrap();
    }

    fn update(&mut self, data: &Vec<u8>) {
        self.data = data.clone();
    }
    
    fn insert_stream(&mut self, name: &Vec<&QueryToken>, data: &Vec<u8>) {
        // Traverse as deep as possible
        let mut deepest = self.search_by_label_stream(name);
        for entry in name {
            deepest = deepest.insert(entry, Vec::new());
        }
        deepest.update(data);
    }
}



fn parse_label(buf: &Vec<u8>, start_index: usize) -> Option<(Vec<QueryToken>, usize)> {
    let label_length = buf[start_index];
//    println!("Got label length: {}", label_length);

    if label_length == 0 {
        return None;
    }

    // If the token is compressed
    if label_length & 0b11000000 == 0b11000000 {
        // Then get the pointer to the stream
        let ptr = buf[start_index + 1];
        println!("Got a compressed pointer starting at {}", ptr);
        let (actual_labels, new_pos) = parse_label_stream(buf, ptr as usize); //parse_label(buf, ptr as usize);

            // Return the actual label that the pointer points to, but the index of the octet after the pointer as we need to skip the reference
        return Some((actual_labels, start_index+1));

    } else {
        let mut label_text = "".to_string();
        let mut index = start_index + 1;
        for _x in 0..label_length {
            label_text = format!("{}{}", label_text, buf[index] as char);
            index+=1;
        }

        return Some((vec![QueryToken::new(label_text.as_str())], index));
    }
}

fn parse_label_stream(buf: &Vec<u8>, start_index: usize) -> (Vec<QueryToken>, usize) {
    let mut labels: Vec<QueryToken> = Vec::new();
    let mut current_index = start_index;

    loop {
        let possible_label = parse_label(buf, current_index);

        if let Some((next_label, end_pos)) = possible_label {
            labels.extend(next_label);
            current_index = end_pos;

            // The label must have been compressed
            if labels.len() > 1  {
                current_index += 1;
                break;
            }
        } else {
            // Exit when we read a token of length 0, and skip the octet
            current_index += 1;
            break;
        }
    }

    return (labels, current_index);

}

fn socket_read_query(socket: &UdpSocket) -> Query {
    let mut buf = [0; 512];
    println!("recv");
    let (amt, src) = socket.recv_from(&mut buf).expect("No data");
    //println!("Got data: {:?}", &buf.to_vec());
//        println!("Test: {}", String::from_utf8(buf.to_vec()).expect("Not valid"));

    let mut query = Query::new(src);
    query.header.identification = BigEndian::read_u16(&buf[0..2]);// (buf[0] << 1 & buf[1]) as u16;
    query.header.flags = BigEndian::read_u16(&buf[2..4]);
    query.header.question_count = BigEndian::read_u16(&buf[4..6]);
    query.header.answer_count = BigEndian::read_u16(&buf[6..8]);
    query.header.authority_records_count = BigEndian::read_u16(&buf[8..10]);
    query.header.additional_records_count = BigEndian::read_u16(&buf[10..12]);


    let mut label_pos = 12;
    for _question_index in 0..query.header.question_count {
        let mut question = QueryQuestion::default();
        let (name_labels, new_pos) = parse_label_stream(&buf.to_vec(), label_pos);
        label_pos = new_pos;
        question.name = name_labels;
        question.type_ = BigEndian::read_u16(&buf[label_pos..label_pos+2]);
        label_pos += 2;
        question.class = BigEndian::read_u16(&buf[label_pos..label_pos+2]);
        label_pos += 2;

        query.questions.push(question);
    }

    for _answer_index in 0..query.header.answer_count {
        let mut answer = QueryAnswer::default();

        let (name_labels, new_pos) = parse_label_stream(&buf.to_vec(), label_pos);
        answer.name = name_labels;
        label_pos = new_pos;

        /*loop {
            let mut label = "".to_string();
            let label_len = buf[label_pos];
            if label_len & 0b11000000 == 0b11000000 {
                // This part is a pointer
                eprintln!("Compressed labels aren't supported");
            }
            println!("Got answer label: {}", label_len);
            label_pos += 1;

            if label_len == 0 {
                break;
            }

            for _label_char_index in 0..label_len {
                label = format!("{}{}", label, buf[label_pos] as char);
                label_pos += 1;
            }
            answer.name.push(QueryToken::new(label.as_str()));
            
        }*/
        //label_pos+=1;//TODO: why is this needed:
        answer.type_ = BigEndian::read_u16(&buf[label_pos..label_pos+2]);
        label_pos+=2;
        answer.class = BigEndian::read_u16(&buf[label_pos..label_pos+2]);
        label_pos+=2;
        answer.ttl = BigEndian::read_u32(&buf[label_pos..label_pos+4]);
        label_pos+=4;
        answer.rd_length = BigEndian::read_u16(&buf[label_pos..label_pos+2]);
        label_pos+=2;

        println!("Answer: {:?}", answer);
        println!("RD_len: {}", answer.rd_length);

        for _rd_index in 0..answer.rd_length {
            answer.r_data.push(buf[label_pos]);
            label_pos+=1;
        }
        query.answers.push(answer);
    }

    println!("Got query: {:?}", query);

    return query;
}

fn do_stub_resolve(query: &Query, socket: &mut UdpSocket, root_node: &mut CacheNode) {

    // Get the question in the query
    let question = &query.questions[0];
    // Try to find the answer in the cache
    let cached_answer = root_node.search_by_label_stream(&question.name.iter().rev().collect());
    println!("Found answer {:?} in cache", cached_answer);
    // Is the answer complete or partial
    println!("HMMM {:?} == {:?}", cached_answer.label.data, query.questions[0].name[0].data);
    if cached_answer.label.data == query.questions[0].name[0].data {
        println!("Cache hit, replying with cache response");


        let mut response = Query::default();
        response.header.identification = query.header.identification;
        response.header.answer_count = 1;
        response.header.question_count = 1;
        response.questions = query.questions.clone();
        //                         Q Op   A T R Ra Z  Rcd 
        response.header.flags = 0b_1_0000_0_0_0_1_000_0000;

        response.answers.push(QueryAnswer::default());
        response.answers[0].name = query.questions[0].name.clone(); //vec![QueryToken::new("google"), QueryToken::new("com")];
        response.answers[0].type_ = query.questions[0].type_;
        response.answers[0].class = query.questions[0].class;
        response.answers[0].ttl = 100;
        response.answers[0].rd_length = cached_answer.data.len() as u16;
        response.answers[0].r_data = cached_answer.data.clone();

        let mut resp_bytes: Vec<u8> = Vec::new();
        response.write(&mut resp_bytes);
        socket.send_to(&resp_bytes, query.requester).unwrap();
        println!("Replied from cache");
        return;
    } else {
        println!("Cache miss");
    }

    // Convert euqery to byte and forward to local resolver
    let mut req_bytes: Vec<u8> = Vec::new();
    query.write(&mut req_bytes);
    socket.send_to(&req_bytes, SocketAddr::new(IpAddr::V4(Ipv4Addr::new(10, 8, 232, 109)), 53)).unwrap();

    // Wait for reply from local resolver
    let resp_local = socket_read_query(&socket);
    let mut resp_bytes: Vec<u8> = Vec::new();
    resp_local.write(&mut resp_bytes);

    // Save local response into the cache
    let names: Vec<&QueryToken> = resp_local.answers[0].name.iter().rev().collect();
    root_node.insert_stream(&names, &resp_local.answers[0].r_data);
    //println!("New cache state: {:?}", root_node);

    // Send it back to the original sender
    socket.send_to(&resp_bytes, query.requester).unwrap();
}


fn main() -> std::io::Result<()> {
    let mut rootNode = CacheNode::new(QueryToken::new("."), vec![127, 0, 0, 1] );
    {
        println!("Socket create");
        let mut socket = UdpSocket::bind("0.0.0.0:53").expect("Unable to create socket");

        loop {
        let query = socket_read_query(&socket);

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

        response.answers.push(QueryAnswer::default());
        response.answers[0].name = query.questions[0].name.clone(); //vec![QueryToken::new("google"), QueryToken::new("com")];
        response.answers[0].type_ = query.questions[0].type_;
        response.answers[0].class = query.questions[0].class;
        response.answers[0].ttl = 100;
        response.answers[0].rd_length = 4;
        response.answers[0].r_data = vec![101, 202, 123, 111];


        do_stub_resolve(&query, &mut socket, &mut rootNode);
        // Forward to local dns
        /*let mut req_bytes: Vec<u8> = Vec::new();
        query.write(&mut req_bytes);
        socket.send_to(&req_bytes, SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 20)), 53)).unwrap();

        // get response from local dns
        let resp_local = socket_read_query(&socket);
        let mut resp_bytes: Vec<u8> = Vec::new();
        resp_local.write(&mut resp_bytes);
        socket.send_to(&resp_bytes, query.requester).unwrap();*/

        }
    } // the socket is closed here
    Ok(())
}
