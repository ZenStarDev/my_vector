use my_vector::Vec;

fn main() {
    let mut v = Vec::new();

    v.push(1);
    v.push(2);
    v.push(3);

    println!("len: {}, cap: {}", v.len(), v.capacity());
    println!("first: {}", v[0]);
    println!("last: {:?}", v.pop());

    if let Some(x) = v.get_mut(0) {
        *x *= 10;
    }

    for x in &v {
        println!("{x}");
    }

    v.clear();
    println!("after clear -> len: {}, cap: {}", v.len(), v.capacity());

    v.extend([100, 200, 300]);
    println!("extended: {:?}", v.as_slice());
}
